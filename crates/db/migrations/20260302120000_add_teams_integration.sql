-- Maps a Teams channel to a local project + repos
CREATE TABLE teams_channel_mappings (
    id                BLOB PRIMARY KEY,       -- UUID
    teams_channel_id  TEXT NOT NULL UNIQUE,   -- e.g. "19:xxxx@thread.tacv2"
    teams_channel_name TEXT,                  -- Human-readable channel name
    teams_service_url TEXT NOT NULL,          -- e.g. "https://smba.trafficmanager.net/..."
    conversation_ref  TEXT NOT NULL DEFAULT '{}',  -- JSON for proactive messaging
    project_id        BLOB,                   -- FK -> projects.id (optional)
    label             TEXT,                   -- Custom label
    created_at        TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- Repos associated with a channel (many-to-many)
CREATE TABLE teams_channel_repos (
    id                 BLOB PRIMARY KEY,
    channel_mapping_id BLOB NOT NULL REFERENCES teams_channel_mappings(id) ON DELETE CASCADE,
    repo_id            BLOB NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    target_branch      TEXT NOT NULL DEFAULT 'main',
    UNIQUE(channel_mapping_id, repo_id)
);

-- Tracks the Teams conversation for each workspace (for reply threading)
CREATE TABLE teams_conversations (
    id                   BLOB PRIMARY KEY,
    workspace_id         BLOB NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_mapping_id   BLOB NOT NULL REFERENCES teams_channel_mappings(id) ON DELETE CASCADE,
    conversation_ref     TEXT NOT NULL,       -- JSON Bot Framework conversation reference
    reply_to_activity_id TEXT,                -- activity_id of the bot's first reply (threading)
    teams_activity_id    TEXT,                -- dedup: original Teams activity id
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE UNIQUE INDEX idx_tc_workspace ON teams_conversations(workspace_id);
CREATE UNIQUE INDEX idx_tc_activity ON teams_conversations(teams_activity_id);
