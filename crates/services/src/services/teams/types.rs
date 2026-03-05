use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

// ── Inbound Activity from Bot Framework ─────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsActivity {
    pub r#type: String,
    pub id: Option<String>,
    pub text: Option<String>,
    pub from: ChannelAccount,
    pub conversation: ConversationAccount,
    pub recipient: Option<ChannelAccount>,
    pub service_url: Option<String>,
    pub channel_data: Option<Value>,
    pub value: Option<Value>,
    pub entities: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAccount {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "aadObjectId")]
    pub aad_object_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAccount {
    pub id: String,
    pub name: Option<String>,
    pub conversation_type: Option<String>,
    pub is_group: Option<bool>,
    pub tenant_id: Option<String>,
}

// ── Parsed Command ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TeamsCommand {
    Help,
    List,
    Status,
    Diff,
    Branch,
    Done,
    DoneMerge,
    DonePr,
    Push,
    Cancel,
    Retry,
    Approve,
    Reject { reason: Option<String> },
    Plan { prompt: String },
    CustomStart { name: String },
    CustomStop { name: String },
    CustomRun { name: String },
    FreeText { text: String },
}

// ── Outbound Message Content ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TeamsMessageContent {
    Text(String),
    AdaptiveCard(Value),
}

// ── Bot Framework Outbound Payloads ─────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BotActivity {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<BotAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BotAttachment {
    pub content_type: String,
    pub content: Value,
}

// ── OAuth Token Response ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

// ── JWKS / OpenID Config ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpenIdConfig {
    pub jwks_uri: String,
    pub issuer: String,
}

#[derive(Debug, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JwkKey>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwkKey {
    pub kid: Option<String>,
    pub kty: String,
    pub n: Option<String>,
    pub e: Option<String>,
    #[serde(rename = "use")]
    pub key_use: Option<String>,
    pub x5c: Option<Vec<String>>,
}

// ── Error Types ─────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TeamsError {
    #[error("Teams integration not enabled")]
    NotEnabled,
    #[error("Missing bot credentials")]
    MissingCredentials,
    #[error("JWT validation failed: {0}")]
    JwtValidation(String),
    #[error("OAuth token error: {0}")]
    OAuthError(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Service URL validation failed: {0}")]
    InvalidServiceUrl(String),
    #[error("Empty prompt")]
    EmptyPrompt,
    #[error("Prompt too long (max {0} chars)")]
    PromptTooLong(usize),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ── API Types (for management endpoints) ────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct TeamsConfigResponse {
    pub enabled: bool,
    pub bot_app_id: Option<String>,
    /// Always None in responses — password is write-only
    pub bot_app_password: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateTeamsConfigRequest {
    pub enabled: Option<bool>,
    pub bot_app_id: Option<String>,
    pub bot_app_password: Option<String>,
}
