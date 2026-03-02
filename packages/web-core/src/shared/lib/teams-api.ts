import { handleApiResponse } from '@/shared/lib/api';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';

const makeRequest = async (url: string, options: RequestInit = {}) => {
  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }
  return makeLocalApiRequest(url, { ...options, headers });
};

// ── Types ──────────────────────────────────────────────────────────────────

export interface TeamsConfigResponse {
  enabled: boolean;
  bot_app_id: string | null;
  bot_app_password: string | null; // always null in GET responses
}

export interface UpdateTeamsConfigRequest {
  enabled?: boolean;
  bot_app_id?: string | null;
  bot_app_password?: string | null;
}

export interface TeamsChannelMapping {
  id: string;
  teams_channel_id: string;
  teams_channel_name: string | null;
  teams_service_url: string;
  project_id: string | null;
  label: string | null;
  created_at: string;
  updated_at: string;
}

export interface TeamsChannelRepo {
  id: string;
  channel_mapping_id: string;
  repo_id: string;
  target_branch: string;
}

export interface CreateChannelMappingRequest {
  teams_channel_id: string;
  teams_channel_name?: string | null;
  teams_service_url: string;
  project_id?: string | null;
  label?: string | null;
}

export interface UpdateChannelMappingRequest {
  teams_channel_name?: string | null;
  teams_service_url?: string;
  project_id?: string | null;
  label?: string | null;
}

export interface AddChannelRepoRequest {
  repo_id: string;
  target_branch: string;
}

// ── API ────────────────────────────────────────────────────────────────────

export const teamsApi = {
  // Config
  getConfig: async (): Promise<TeamsConfigResponse> => {
    const response = await makeRequest('/api/teams/config');
    return handleApiResponse<TeamsConfigResponse>(response);
  },

  updateConfig: async (
    body: UpdateTeamsConfigRequest
  ): Promise<TeamsConfigResponse> => {
    const response = await makeRequest('/api/teams/config', {
      method: 'PUT',
      body: JSON.stringify(body),
    });
    return handleApiResponse<TeamsConfigResponse>(response);
  },

  // Channel mappings
  listChannels: async (): Promise<TeamsChannelMapping[]> => {
    const response = await makeRequest('/api/teams/channels');
    return handleApiResponse<TeamsChannelMapping[]>(response);
  },

  createChannel: async (
    body: CreateChannelMappingRequest
  ): Promise<TeamsChannelMapping> => {
    const response = await makeRequest('/api/teams/channels', {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return handleApiResponse<TeamsChannelMapping>(response);
  },

  updateChannel: async (
    id: string,
    body: UpdateChannelMappingRequest
  ): Promise<TeamsChannelMapping> => {
    const response = await makeRequest(
      `/api/teams/channels/${encodeURIComponent(id)}`,
      {
        method: 'PUT',
        body: JSON.stringify(body),
      }
    );
    return handleApiResponse<TeamsChannelMapping>(response);
  },

  deleteChannel: async (id: string): Promise<void> => {
    const response = await makeRequest(
      `/api/teams/channels/${encodeURIComponent(id)}`,
      { method: 'DELETE' }
    );
    await handleApiResponse(response);
  },

  // Channel repos
  listChannelRepos: async (channelId: string): Promise<TeamsChannelRepo[]> => {
    const response = await makeRequest(
      `/api/teams/channels/${encodeURIComponent(channelId)}/repos`
    );
    return handleApiResponse<TeamsChannelRepo[]>(response);
  },

  addChannelRepo: async (
    channelId: string,
    body: AddChannelRepoRequest
  ): Promise<TeamsChannelRepo> => {
    const response = await makeRequest(
      `/api/teams/channels/${encodeURIComponent(channelId)}/repos`,
      {
        method: 'POST',
        body: JSON.stringify(body),
      }
    );
    return handleApiResponse<TeamsChannelRepo>(response);
  },

  removeChannelRepo: async (
    channelId: string,
    repoId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/teams/channels/${encodeURIComponent(channelId)}/repos/${encodeURIComponent(repoId)}`,
      { method: 'DELETE' }
    );
    await handleApiResponse(response);
  },
};
