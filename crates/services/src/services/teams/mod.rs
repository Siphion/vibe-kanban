pub mod cards;
pub mod types;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use db::DBService;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;
pub use types::*;
use uuid::Uuid;

use crate::services::config::TeamsConfig;

// ── Constants ───────────────────────────────────────────────────────────────

const BOT_FRAMEWORK_OPENID_URL: &str =
    "https://login.botframework.com/v1/.well-known/openidconfiguration";
const BOT_OAUTH_URL: &str = "https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token";
const BOT_OAUTH_SCOPE: &str = "https://api.botframework.com/.default";

const JWKS_CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour
const TOKEN_CACHE_MARGIN: Duration = Duration::from_secs(300); // refresh 5 min before expiry

const MAX_PROMPT_LENGTH: usize = 10_000;

/// Allowed Bot Framework service URL domain suffixes.
const ALLOWED_SERVICE_DOMAINS: &[&str] = &[
    ".botframework.com",
    ".trafficmanager.net",
    ".servicebus.windows.net",
];

// ── Cached Token ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct CachedBotToken {
    token: String,
    expires_at: Instant,
}

#[derive(Clone)]
struct CachedJwks {
    keys: Vec<types::JwkKey>,
    fetched_at: Instant,
}

// ── TeamsService ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TeamsService {
    http: Client,
    token_cache: Arc<Mutex<Option<CachedBotToken>>>,
    jwks_cache: Arc<Mutex<Option<CachedJwks>>>,
    db: DBService,
}

impl TeamsService {
    pub fn new(db: DBService) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            token_cache: Arc::new(Mutex::new(None)),
            jwks_cache: Arc::new(Mutex::new(None)),
            db,
        }
    }

    pub fn db(&self) -> &DBService {
        &self.db
    }

    // ── Command Parsing ─────────────────────────────────────────────────

    /// Strip `<at>...</at>` mention tags from Teams message text.
    pub fn strip_mentions(text: &str) -> String {
        // Teams wraps bot mentions in <at>BotName</at> tags
        let mut result = text.to_string();
        while let Some(start) = result.find("<at>") {
            if let Some(end) = result[start..].find("</at>") {
                result = format!("{}{}", &result[..start], &result[start + end + 5..]);
            } else {
                break;
            }
        }
        result.trim().to_string()
    }

    /// Parse a command from stripped message text.
    pub fn parse_command(text: &str) -> TeamsCommand {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return TeamsCommand::Help;
        }

        // Split on first whitespace
        let (first, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((f, r)) => (f, Some(r.trim())),
            None => (trimmed, None),
        };

        match first.to_lowercase().as_str() {
            "help" => TeamsCommand::Help,
            "list" => TeamsCommand::List,
            "status" => TeamsCommand::Status,
            "diff" => TeamsCommand::Diff,
            "branch" => TeamsCommand::Branch,
            "done" => TeamsCommand::Done,
            "donemerge" => TeamsCommand::DoneMerge,
            "donepr" => TeamsCommand::DonePr,
            "push" => TeamsCommand::Push,
            "cancel" => TeamsCommand::Cancel,
            "retry" => TeamsCommand::Retry,
            "approve" => TeamsCommand::Approve,
            "reject" => TeamsCommand::Reject {
                reason: rest.filter(|r| !r.is_empty()).map(|r| r.to_string()),
            },
            "plan" => {
                let prompt = rest.unwrap_or("").to_string();
                if prompt.is_empty() {
                    TeamsCommand::Help // plan without text → show help
                } else {
                    TeamsCommand::Plan { prompt }
                }
            }
            _ => TeamsCommand::FreeText {
                text: trimmed.to_string(),
            },
        }
    }

    /// Sanitize a prompt from Teams.
    pub fn sanitize_prompt(text: &str) -> Result<String, TeamsError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TeamsError::EmptyPrompt);
        }
        if trimmed.len() > MAX_PROMPT_LENGTH {
            return Err(TeamsError::PromptTooLong(MAX_PROMPT_LENGTH));
        }
        // Remove control characters except newline/carriage return
        let sanitized: String = trimmed
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r')
            .collect();
        Ok(sanitized)
    }

    // ── Service URL Validation ──────────────────────────────────────────

    /// Validate that the service URL is from a trusted Microsoft domain.
    pub fn validate_service_url(url: &str) -> Result<(), TeamsError> {
        let parsed =
            url::Url::parse(url).map_err(|e| TeamsError::InvalidServiceUrl(e.to_string()))?;

        if parsed.scheme() != "https" {
            return Err(TeamsError::InvalidServiceUrl(
                "service URL must use HTTPS".to_string(),
            ));
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| TeamsError::InvalidServiceUrl("missing host".to_string()))?;

        let is_allowed = ALLOWED_SERVICE_DOMAINS
            .iter()
            .any(|domain| host.ends_with(domain));

        if !is_allowed {
            return Err(TeamsError::InvalidServiceUrl(format!(
                "untrusted service URL domain: {}",
                host
            )));
        }

        Ok(())
    }

    // ── JWT Validation (Inbound) ────────────────────────────────────────

    /// Validate the JWT token from an incoming Bot Framework webhook request.
    pub async fn validate_incoming_token(
        &self,
        auth_header: &str,
        bot_app_id: &str,
    ) -> Result<(), TeamsError> {
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| TeamsError::JwtValidation("missing Bearer prefix".to_string()))?;

        let jwks = self.get_jwks().await?;

        // Decode the header to find the key ID
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| TeamsError::JwtValidation(format!("invalid JWT header: {}", e)))?;

        let kid = header
            .kid
            .ok_or_else(|| TeamsError::JwtValidation("missing kid in JWT header".to_string()))?;

        // Find matching key
        let jwk = jwks
            .iter()
            .find(|k| k.kid.as_deref() == Some(&kid))
            .ok_or_else(|| {
                TeamsError::JwtValidation("no matching key found in JWKS".to_string())
            })?;

        // Build decoding key from RSA components
        let n = jwk
            .n
            .as_ref()
            .ok_or_else(|| TeamsError::JwtValidation("missing n in JWK".to_string()))?;
        let e = jwk
            .e
            .as_ref()
            .ok_or_else(|| TeamsError::JwtValidation("missing e in JWK".to_string()))?;

        let decoding_key = DecodingKey::from_rsa_components(n, e)
            .map_err(|e| TeamsError::JwtValidation(format!("invalid RSA key: {}", e)))?;

        // Set up validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[bot_app_id]);
        validation.set_issuer(&["https://api.botframework.com"]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        // Decode and validate
        #[derive(Deserialize)]
        struct Claims {}

        decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| TeamsError::JwtValidation(format!("JWT verification failed: {}", e)))?;

        Ok(())
    }

    /// Fetch and cache JWKS from Bot Framework OpenID configuration.
    async fn get_jwks(&self) -> Result<Vec<JwkKey>, TeamsError> {
        let mut cache = self.jwks_cache.lock().await;

        if let Some(ref cached) = *cache
            && cached.fetched_at.elapsed() < JWKS_CACHE_TTL
        {
            return Ok(cached.keys.clone());
        }

        // Fetch OpenID config to get JWKS URI
        let openid_config: OpenIdConfig = self
            .http
            .get(BOT_FRAMEWORK_OPENID_URL)
            .send()
            .await
            .map_err(|e| {
                TeamsError::JwtValidation(format!("failed to fetch OpenID config: {}", e))
            })?
            .json()
            .await
            .map_err(|e| TeamsError::JwtValidation(format!("invalid OpenID config: {}", e)))?;

        // Fetch JWKS
        let jwks: JwksResponse = self
            .http
            .get(&openid_config.jwks_uri)
            .send()
            .await
            .map_err(|e| TeamsError::JwtValidation(format!("failed to fetch JWKS: {}", e)))?
            .json()
            .await
            .map_err(|e| TeamsError::JwtValidation(format!("invalid JWKS: {}", e)))?;

        *cache = Some(CachedJwks {
            keys: jwks.keys.clone(),
            fetched_at: Instant::now(),
        });

        Ok(jwks.keys)
    }

    // ── OAuth2 Token (Outbound) ─────────────────────────────────────────

    /// Get a Bot Framework OAuth token for outbound API calls.
    pub async fn get_bot_token(
        &self,
        bot_app_id: &str,
        bot_app_password: &str,
    ) -> Result<String, TeamsError> {
        let mut cache = self.token_cache.lock().await;

        if let Some(ref cached) = *cache
            && Instant::now() + TOKEN_CACHE_MARGIN < cached.expires_at
        {
            return Ok(cached.token.clone());
        }

        let response: OAuthTokenResponse = self
            .http
            .post(BOT_OAUTH_URL)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", bot_app_id),
                ("client_secret", bot_app_password),
                ("scope", BOT_OAUTH_SCOPE),
            ])
            .send()
            .await
            .map_err(|e| TeamsError::OAuthError(format!("token request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| TeamsError::OAuthError(format!("invalid token response: {}", e)))?;

        let cached_token = CachedBotToken {
            token: response.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(response.expires_in),
        };
        *cache = Some(cached_token);

        Ok(response.access_token)
    }

    // ── Send Message / Adaptive Card ────────────────────────────────────

    /// Send a reply to a Teams conversation.
    ///
    /// Returns the activity ID of the sent message (for threading).
    pub async fn send_reply(
        &self,
        service_url: &str,
        conversation_id: &str,
        reply_to_id: Option<&str>,
        content: TeamsMessageContent,
        bot_app_id: &str,
        bot_app_password: &str,
    ) -> Result<String, TeamsError> {
        Self::validate_service_url(service_url)?;

        let token = self.get_bot_token(bot_app_id, bot_app_password).await?;

        let activity = match content {
            TeamsMessageContent::Text(text) => BotActivity {
                r#type: "message".to_string(),
                text: Some(text),
                attachments: None,
                reply_to_id: reply_to_id.map(|s| s.to_string()),
            },
            TeamsMessageContent::AdaptiveCard(card) => BotActivity {
                r#type: "message".to_string(),
                text: None,
                attachments: Some(vec![BotAttachment {
                    content_type: "application/vnd.microsoft.card.adaptive".to_string(),
                    content: card,
                }]),
                reply_to_id: reply_to_id.map(|s| s.to_string()),
            },
        };

        let url = format!(
            "{}v3/conversations/{}/activities",
            service_url.trim_end_matches('/').to_owned() + "/",
            conversation_id
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&activity)
            .send()
            .await
            .map_err(|e| TeamsError::HttpError(format!("send reply failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(TeamsError::HttpError(format!(
                "Bot Connector returned {}: {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct ActivityResponse {
            id: Option<String>,
        }

        let resp: ActivityResponse = response
            .json()
            .await
            .map_err(|e| TeamsError::HttpError(format!("invalid activity response: {}", e)))?;

        Ok(resp.id.unwrap_or_default())
    }

    /// Convenience: send a text reply.
    pub async fn send_text_reply(
        &self,
        service_url: &str,
        conversation_id: &str,
        reply_to_id: Option<&str>,
        text: &str,
        config: &TeamsConfig,
    ) -> Result<String, TeamsError> {
        let (app_id, app_pass) = self.get_credentials(config)?;
        self.send_reply(
            service_url,
            conversation_id,
            reply_to_id,
            TeamsMessageContent::Text(text.to_string()),
            &app_id,
            &app_pass,
        )
        .await
    }

    /// Convenience: send an Adaptive Card reply.
    pub async fn send_card_reply(
        &self,
        service_url: &str,
        conversation_id: &str,
        reply_to_id: Option<&str>,
        card: serde_json::Value,
        config: &TeamsConfig,
    ) -> Result<String, TeamsError> {
        let (app_id, app_pass) = self.get_credentials(config)?;
        self.send_reply(
            service_url,
            conversation_id,
            reply_to_id,
            TeamsMessageContent::AdaptiveCard(card),
            &app_id,
            &app_pass,
        )
        .await
    }

    // ── Notification Helpers ────────────────────────────────────────────

    /// Notify a Teams channel that a workspace was completed.
    pub async fn notify_workspace_completed(
        &self,
        config: &TeamsConfig,
        workspace_id: Uuid,
        workspace_name: &str,
        status: &str,
    ) -> Result<(), TeamsError> {
        if !config.enabled {
            return Ok(());
        }

        let conversation =
            db::models::teams::TeamsConversation::find_by_workspace_id(&self.db.pool, workspace_id)
                .await?;

        let conversation = match conversation {
            Some(c) => c,
            None => return Ok(()), // No Teams conversation for this workspace
        };

        let mapping = db::models::teams::TeamsChannelMapping::find_by_id(
            &self.db.pool,
            conversation.channel_mapping_id,
        )
        .await?;

        let mapping = match mapping {
            Some(m) => m,
            None => return Ok(()),
        };

        let card = cards::build_workspace_completed_card(workspace_name, status, None, None, None);

        self.send_card_reply(
            &mapping.teams_service_url,
            &conversation.conversation_ref,
            conversation.reply_to_activity_id.as_deref(),
            card,
            config,
        )
        .await?;

        Ok(())
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn get_credentials(&self, config: &TeamsConfig) -> Result<(String, String), TeamsError> {
        let app_id = config
            .bot_app_id
            .as_ref()
            .ok_or(TeamsError::MissingCredentials)?
            .clone();
        let app_pass = config
            .bot_app_password
            .as_ref()
            .ok_or(TeamsError::MissingCredentials)?
            .clone();
        Ok((app_id, app_pass))
    }
}
