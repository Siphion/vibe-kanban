# Custom Patches

This fork maintains a set of patches on top of upstream
[BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban) `main`.

All patches live on the `custom/patches` branch and are pure insertions
(no upstream lines modified) to minimise rebase conflicts.

## Patch List

### 1. `$VK-URL$` marker for explicit URL detection

**Files:**
- `frontend/src/hooks/useDevserverUrl.ts`
- `frontend/src/components/ui-new/hooks/usePreviewUrl.ts`

Adds a high-priority check for the `$VK-URL$<url>$VK-URL$` marker in
process output. When a dev server or tool prints this marker, the URL
is extracted without relying on the generic localhost/IP heuristics.

### 2. HTTP URL templates as custom editor command

**Files:**
- `crates/services/src/services/config/editor/mod.rs`
- `crates/server/src/routes/projects.rs`
- `crates/server/src/routes/repo.rs`
- `crates/server/src/routes/task_attempts.rs`

When the editor type is `Custom` and the custom command starts with
`http://` or `https://`, the command is treated as a URL template
instead of a local executable. Two placeholders are available:

- `{path}` — always resolves to the workspace/repo **folder** (never a
  file), so `https://host:port/?folder={path}` works reliably with
  code-server and similar tools.
- `{file}` — resolves to the full file path (`folder` + relative file)
  when the user clicks "open file"; empty string when opening a
  project/repo folder.

The resulting URL is returned to the frontend, which opens it in a new
browser tab.

## Updating from Upstream

```bash
./scripts/sync-upstream.sh
```

This fetches upstream `main` and rebases `custom/patches` on top.
If there are conflicts, resolve them and run `git rebase --continue`.

## Adding New Patches

1. Check out `custom/patches`.
2. Make your changes as pure insertions whenever possible.
3. Commit with a descriptive message.
4. Update this file with a summary of the new patch.
