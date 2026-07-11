# Example Plugin

Template for ArcHive frontend plugins.

## Install

Already included in the repo. For a third-party plugin:

```bash
git clone <repo-url> plugins/my-plugin
bun run plugins:generate
```

## Files

- `plugin.json` — manifest (id, name, version, entry)
- `index.ts` — calls `register()` to add browse sites and settings panels

See [docs/plugins.md](../../docs/plugins.md).
