import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PlusIcon, SpinnerIcon, TrashIcon } from '@phosphor-icons/react';
import { useUserSystem } from '@/shared/hooks/useUserSystem';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';
import { teamsApi, type TeamsChannelMapping } from '@/shared/lib/teams-api';
import { repoApi } from '@/shared/lib/api';
import type { Repo } from 'shared/types';
import {
  SettingsCard,
  SettingsCheckbox,
  SettingsField,
  SettingsInput,
  SettingsSaveBar,
} from './SettingsComponents';
import { useSettingsDirty } from './SettingsDirtyContext';

const TEAMS_CONFIG_KEY = ['teams', 'config'] as const;
const TEAMS_CHANNELS_KEY = ['teams', 'channels'] as const;

export function TeamsSettingsSectionContent() {
  const { t } = useTranslation(['settings', 'common']);
  const { setDirty: setContextDirty } = useSettingsDirty();
  const userSystem = useUserSystem();
  const { config, loading: configLoading } = userSystem;
  const queryClient = useQueryClient();

  // ── Bot config state ─────────────────────────────────────────────────
  const [enabled, setEnabled] = useState(false);
  const [botAppId, setBotAppId] = useState('');
  const [botAppPassword, setBotAppPassword] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  // Track original values for dirty checking
  const [originalEnabled, setOriginalEnabled] = useState(false);
  const [originalBotAppId, setOriginalBotAppId] = useState('');

  // ── Load teams config ────────────────────────────────────────────────
  const { data: teamsConfig, isLoading: teamsConfigLoading } = useQuery({
    queryKey: TEAMS_CONFIG_KEY,
    queryFn: () => teamsApi.getConfig(),
  });

  useEffect(() => {
    if (teamsConfig) {
      setEnabled(teamsConfig.enabled);
      setBotAppId(teamsConfig.bot_app_id ?? '');
      setBotAppPassword('');
      setOriginalEnabled(teamsConfig.enabled);
      setOriginalBotAppId(teamsConfig.bot_app_id ?? '');
    }
  }, [teamsConfig]);

  const hasUnsavedChanges = useMemo(() => {
    if (!teamsConfig) return false;
    return (
      enabled !== originalEnabled ||
      botAppId !== originalBotAppId ||
      botAppPassword.length > 0
    );
  }, [
    enabled,
    botAppId,
    botAppPassword,
    originalEnabled,
    originalBotAppId,
    teamsConfig,
  ]);

  useEffect(() => {
    setContextDirty('teams', hasUnsavedChanges);
    return () => setContextDirty('teams', false);
  }, [hasUnsavedChanges, setContextDirty]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(false);
    try {
      const body: Record<string, unknown> = {
        enabled,
        bot_app_id: botAppId || null,
      };
      if (botAppPassword.length > 0) {
        body.bot_app_password = botAppPassword;
      }
      await teamsApi.updateConfig(body);
      await queryClient.invalidateQueries({ queryKey: TEAMS_CONFIG_KEY });
      setBotAppPassword('');
      setSuccess(true);
      setTimeout(() => setSuccess(false), 3000);
    } catch {
      setError(
        t('settings.teams.save.error', 'Failed to save Teams configuration')
      );
    } finally {
      setSaving(false);
    }
  };

  const handleDiscard = () => {
    if (teamsConfig) {
      setEnabled(teamsConfig.enabled);
      setBotAppId(teamsConfig.bot_app_id ?? '');
      setBotAppPassword('');
    }
  };

  // ── Channels ─────────────────────────────────────────────────────────
  const { data: channels = [], isLoading: channelsLoading } = useQuery({
    queryKey: TEAMS_CHANNELS_KEY,
    queryFn: () => teamsApi.listChannels(),
    enabled: teamsConfig?.enabled ?? false,
  });

  const { data: repos = [] } = useQuery({
    queryKey: ['repos'],
    queryFn: () => repoApi.list(),
    enabled: teamsConfig?.enabled ?? false,
  });

  const deleteChannelMutation = useMutation({
    mutationFn: (id: string) => teamsApi.deleteChannel(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TEAMS_CHANNELS_KEY });
    },
  });

  // ── Loading ──────────────────────────────────────────────────────────
  if (configLoading || teamsConfigLoading) {
    return (
      <div className="flex items-center justify-center py-8 gap-2">
        <SpinnerIcon
          className="size-icon-lg animate-spin text-brand"
          weight="bold"
        />
        <span className="text-normal">{t('settings.general.loading')}</span>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="py-8">
        <div className="bg-error/10 border border-error/50 rounded-sm p-4 text-error">
          {t('settings.general.loadError')}
        </div>
      </div>
    );
  }

  return (
    <>
      {error && (
        <div className="bg-error/10 border border-error/50 rounded-sm p-4 text-error">
          {error}
        </div>
      )}

      {success && (
        <div className="bg-success/10 border border-success/50 rounded-sm p-4 text-success font-medium">
          {t('settings.teams.save.success', 'Teams configuration saved!')}
        </div>
      )}

      {/* Bot Configuration */}
      <SettingsCard
        title={t('settings.teams.bot.title', 'Bot Configuration')}
        description={t(
          'settings.teams.bot.description',
          'Configure your Azure Bot Service credentials for Teams integration.'
        )}
      >
        <SettingsCheckbox
          id="teams-enabled"
          label={t(
            'settings.teams.bot.enabled.label',
            'Enable Teams integration'
          )}
          description={t(
            'settings.teams.bot.enabled.helper',
            'When enabled, the bot listens for mentions in configured Teams channels.'
          )}
          checked={enabled}
          onChange={setEnabled}
        />

        {enabled && (
          <div className="space-y-3 mt-2">
            <SettingsField
              label={t('settings.teams.bot.appId.label', 'Bot App ID')}
              description={t(
                'settings.teams.bot.appId.helper',
                'The Application (client) ID from your Azure Bot registration.'
              )}
            >
              <SettingsInput
                value={botAppId}
                onChange={setBotAppId}
                placeholder={t(
                  'settings.teams.bot.appId.placeholder',
                  'e.g., 12345678-abcd-1234-abcd-123456789012'
                )}
              />
            </SettingsField>

            <SettingsField
              label={t(
                'settings.teams.bot.appPassword.label',
                'Bot App Password'
              )}
              description={t(
                'settings.teams.bot.appPassword.helper',
                'The client secret from your Azure Bot registration. Stored securely and never displayed.'
              )}
            >
              <input
                type="password"
                value={botAppPassword}
                onChange={(e) => setBotAppPassword(e.target.value)}
                placeholder={
                  teamsConfig?.bot_app_id
                    ? t(
                        'settings.teams.bot.appPassword.placeholderSet',
                        '••••••••  (leave empty to keep current)'
                      )
                    : t(
                        'settings.teams.bot.appPassword.placeholder',
                        'Enter client secret'
                      )
                }
                className="w-full bg-secondary border border-border rounded-sm px-base py-half text-sm text-high placeholder:text-low placeholder:opacity-80 focus:outline-none focus:ring-1 focus:ring-brand"
              />
            </SettingsField>

            <SettingsField
              label={t('settings.teams.bot.webhookUrl.label', 'Webhook URL')}
              description={t(
                'settings.teams.bot.webhookUrl.helper',
                'Set this as the Messaging endpoint in your Azure Bot configuration.'
              )}
            >
              <div className="bg-secondary border border-border rounded-sm px-base py-half font-mono text-sm text-low select-all">
                {window.location.origin}/api/teams/webhook
              </div>
            </SettingsField>
          </div>
        )}
      </SettingsCard>

      {/* Channel Mappings */}
      {enabled && (
        <ChannelMappingsSection
          channels={channels}
          channelsLoading={channelsLoading}
          repos={repos}
          onDeleteChannel={(id) => deleteChannelMutation.mutate(id)}
          deletingChannel={deleteChannelMutation.isPending}
        />
      )}

      <SettingsSaveBar
        show={hasUnsavedChanges}
        saving={saving}
        onSave={() => void handleSave()}
        onDiscard={handleDiscard}
      />
    </>
  );
}

// ── Channel Mappings Section ─────────────────────────────────────────────────

function ChannelMappingsSection({
  channels,
  channelsLoading,
  repos,
  onDeleteChannel,
  deletingChannel,
}: {
  channels: TeamsChannelMapping[];
  channelsLoading: boolean;
  repos: Repo[];
  onDeleteChannel: (id: string) => void;
  deletingChannel: boolean;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const queryClient = useQueryClient();
  const [showAddForm, setShowAddForm] = useState(false);
  const [newChannelId, setNewChannelId] = useState('');
  const [newChannelName, setNewChannelName] = useState('');
  const [newServiceUrl, setNewServiceUrl] = useState('');
  const [newLabel, setNewLabel] = useState('');
  const [addError, setAddError] = useState<string | null>(null);

  const createChannelMutation = useMutation({
    mutationFn: () =>
      teamsApi.createChannel({
        teams_channel_id: newChannelId,
        teams_channel_name: newChannelName || null,
        teams_service_url: newServiceUrl,
        label: newLabel || null,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TEAMS_CHANNELS_KEY });
      setShowAddForm(false);
      setNewChannelId('');
      setNewChannelName('');
      setNewServiceUrl('');
      setNewLabel('');
      setAddError(null);
    },
    onError: (e) => {
      setAddError(e instanceof Error ? e.message : String(e));
    },
  });

  return (
    <SettingsCard
      title={t('settings.teams.channels.title', 'Channel Mappings')}
      description={t(
        'settings.teams.channels.description',
        'Map Teams channels to repositories. The bot will create workspaces for the configured repos when mentioned in a channel.'
      )}
      headerAction={
        <PrimaryButton
          variant="secondary"
          value={t('settings.teams.channels.add', 'Add Channel')}
          onClick={() => {
            setAddError(null);
            setShowAddForm((v) => !v);
          }}
        >
          <PlusIcon className="size-icon-xs mr-1" weight="bold" />
        </PrimaryButton>
      }
    >
      {showAddForm && (
        <div className="border border-border rounded-sm bg-secondary/40 p-4 space-y-3">
          <SettingsField
            label={t(
              'settings.teams.channels.channelId.label',
              'Teams Channel ID'
            )}
            description={t(
              'settings.teams.channels.channelId.helper',
              'The channel ID from Teams (e.g., 19:xxxx@thread.tacv2).'
            )}
          >
            <SettingsInput
              value={newChannelId}
              onChange={setNewChannelId}
              placeholder="19:xxxx@thread.tacv2"
            />
          </SettingsField>

          <SettingsField
            label={t(
              'settings.teams.channels.channelName.label',
              'Channel Name'
            )}
            description={t(
              'settings.teams.channels.channelName.helper',
              'A friendly name for this channel (optional).'
            )}
          >
            <SettingsInput
              value={newChannelName}
              onChange={setNewChannelName}
              placeholder={t(
                'settings.teams.channels.channelName.placeholder',
                'e.g., #dev-tasks'
              )}
            />
          </SettingsField>

          <SettingsField
            label={t('settings.teams.channels.serviceUrl.label', 'Service URL')}
            description={t(
              'settings.teams.channels.serviceUrl.helper',
              'The Bot Framework service URL (populated automatically on first message).'
            )}
          >
            <SettingsInput
              value={newServiceUrl}
              onChange={setNewServiceUrl}
              placeholder="https://smba.trafficmanager.net/..."
            />
          </SettingsField>

          <SettingsField
            label={t('settings.teams.channels.label.label', 'Label')}
            description={t(
              'settings.teams.channels.label.helper',
              'Optional label for this mapping.'
            )}
          >
            <SettingsInput
              value={newLabel}
              onChange={setNewLabel}
              placeholder={t(
                'settings.teams.channels.label.placeholder',
                'e.g., Frontend team'
              )}
            />
          </SettingsField>

          {addError && <p className="text-sm text-error">{addError}</p>}

          <div className="flex items-center gap-2">
            <PrimaryButton
              value={t('settings.teams.channels.create', 'Create')}
              onClick={() => createChannelMutation.mutate()}
              disabled={
                !newChannelId.trim() ||
                !newServiceUrl.trim() ||
                createChannelMutation.isPending
              }
              actionIcon={
                createChannelMutation.isPending ? 'spinner' : undefined
              }
            />
            <PrimaryButton
              variant="tertiary"
              value={t('common:buttons.cancel')}
              onClick={() => setShowAddForm(false)}
              disabled={createChannelMutation.isPending}
            />
          </div>
        </div>
      )}

      {channelsLoading && (
        <div className="flex items-center gap-2 text-sm text-low">
          <SpinnerIcon className="size-icon-sm animate-spin" weight="bold" />
          <span>
            {t('settings.teams.channels.loading', 'Loading channels...')}
          </span>
        </div>
      )}

      {!channelsLoading && channels.length === 0 && (
        <div className="rounded-sm border border-border bg-secondary/30 p-3 text-sm text-low">
          {t(
            'settings.teams.channels.empty',
            'No channel mappings configured. Add a channel to get started.'
          )}
        </div>
      )}

      {!channelsLoading &&
        channels.map((channel) => (
          <ChannelMappingRow
            key={channel.id}
            channel={channel}
            repos={repos}
            onDelete={() => onDeleteChannel(channel.id)}
            deleting={deletingChannel}
          />
        ))}
    </SettingsCard>
  );
}

// ── Channel Mapping Row ──────────────────────────────────────────────────────

function ChannelMappingRow({
  channel,
  repos,
  onDelete,
  deleting,
}: {
  channel: TeamsChannelMapping;
  repos: Repo[];
  onDelete: () => void;
  deleting: boolean;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const queryClient = useQueryClient();
  const channelReposKey = ['teams', 'channels', channel.id, 'repos'] as const;

  const { data: channelRepos = [], isLoading: channelReposLoading } = useQuery({
    queryKey: channelReposKey,
    queryFn: () => teamsApi.listChannelRepos(channel.id),
  });

  const [addRepoId, setAddRepoId] = useState('');
  const [addTargetBranch, setAddTargetBranch] = useState('main');

  const addRepoMutation = useMutation({
    mutationFn: () =>
      teamsApi.addChannelRepo(channel.id, {
        repo_id: addRepoId,
        target_branch: addTargetBranch,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: channelReposKey });
      setAddRepoId('');
      setAddTargetBranch('main');
    },
  });

  const removeRepoMutation = useMutation({
    mutationFn: (repoId: string) =>
      teamsApi.removeChannelRepo(channel.id, repoId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: channelReposKey });
    },
  });

  // Repos not yet added to this channel
  const availableRepos = useMemo(() => {
    const assigned = new Set(channelRepos.map((cr) => cr.repo_id));
    return repos.filter((r) => !assigned.has(r.id));
  }, [repos, channelRepos]);

  const getRepoName = useCallback(
    (repoId: string) => repos.find((r) => r.id === repoId)?.name ?? repoId,
    [repos]
  );

  return (
    <div className="rounded-sm border border-border bg-secondary/30 p-3 space-y-2">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium text-high truncate">
            {channel.label ||
              channel.teams_channel_name ||
              channel.teams_channel_id}
          </p>
          <p className="text-xs text-low truncate">
            {channel.teams_channel_id}
          </p>
        </div>
        <button
          onClick={onDelete}
          disabled={deleting}
          className="p-1 rounded-sm text-low hover:text-error transition-colors shrink-0"
          title={t('settings.teams.channels.delete', 'Delete channel mapping')}
        >
          <TrashIcon className="size-icon-sm" weight="bold" />
        </button>
      </div>

      {/* Repos for this channel */}
      <div className="space-y-1">
        <p className="text-xs font-medium text-normal">
          {t('settings.teams.channels.repos.title', 'Repositories')}
        </p>

        {channelReposLoading && (
          <div className="flex items-center gap-1 text-xs text-low">
            <SpinnerIcon className="size-icon-xs animate-spin" weight="bold" />
            <span>
              {t('settings.teams.channels.repos.loading', 'Loading...')}
            </span>
          </div>
        )}

        {!channelReposLoading && channelRepos.length === 0 && (
          <p className="text-xs text-low">
            {t(
              'settings.teams.channels.repos.empty',
              'No repositories assigned.'
            )}
          </p>
        )}

        {channelRepos.map((cr) => (
          <div
            key={cr.id}
            className="flex items-center justify-between gap-2 text-xs bg-panel/50 rounded-sm px-2 py-1"
          >
            <span className="text-normal truncate">
              {getRepoName(cr.repo_id)}{' '}
              <span className="text-low">→ {cr.target_branch}</span>
            </span>
            <button
              onClick={() => removeRepoMutation.mutate(cr.repo_id)}
              disabled={removeRepoMutation.isPending}
              className="p-0.5 text-low hover:text-error transition-colors shrink-0"
            >
              <TrashIcon className="size-icon-xs" weight="bold" />
            </button>
          </div>
        ))}

        {/* Add repo form */}
        {availableRepos.length > 0 && (
          <div className="flex items-center gap-2 pt-1">
            <select
              value={addRepoId}
              onChange={(e) => setAddRepoId(e.target.value)}
              className="flex-1 text-xs rounded-sm border border-border bg-panel px-2 py-1 text-normal"
            >
              <option value="">
                {t(
                  'settings.teams.channels.repos.selectRepo',
                  'Select repository...'
                )}
              </option>
              {availableRepos.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
            <input
              type="text"
              value={addTargetBranch}
              onChange={(e) => setAddTargetBranch(e.target.value)}
              placeholder="main"
              className="w-24 text-xs rounded-sm border border-border bg-panel px-2 py-1 text-normal"
            />
            <button
              onClick={() => addRepoMutation.mutate()}
              disabled={!addRepoId || addRepoMutation.isPending}
              className="p-1 rounded-sm text-brand hover:text-brand/80 transition-colors disabled:opacity-50"
              title={t('settings.teams.channels.repos.add', 'Add repository')}
            >
              <PlusIcon className="size-icon-xs" weight="bold" />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
