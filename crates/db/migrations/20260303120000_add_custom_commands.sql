CREATE TABLE IF NOT EXISTS custom_commands (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    script TEXT NOT NULL,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec'))
);
