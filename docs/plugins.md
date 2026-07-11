# ArcHive plugins

Extend the UI with **Bun + TypeScript** plugins — no Rust recompile required.

Plugins are **frontend-only**. For real scrape/download backends, add a Rust site adapter ([custom-sites.md](custom-sites.md)).

## Install a plugin

```bash
git clone https://github.com/you/my-plugin.git plugins/my-plugin
bun run plugins:generate
bun run dev
```

Each plugin is a folder under [`plugins/`](../plugins/) with a `plugin.json` manifest and `index.ts` entry.

## Create a plugin

### 1. Manifest (`plugin.json`)

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "entry": "index.ts"
}
```

Optional manifest fields (documentation only for v1 — registration happens in `index.ts`):

- `browseSites` — site cards on the Browse hub
- `routes` — custom pages (wired via plugin registry)
- `settingsPanels` — extra Settings sections

### 2. Entry (`index.ts`)

```typescript
import type { ArcHivePlugin } from "@/lib/plugins/types";

const plugin: ArcHivePlugin = {
  id: "my-plugin",
  register(ctx) {
    ctx.addBrowseSite({
      id: "my-tube",
      display_name: "My Tube",
      base_url: "https://example.com",
      supported_kinds: ["search", "video"],
      requires_cookies: false,
    });
    ctx.addSettingsPanel({
      id: "my-plugin-info",
      title: "My Plugin",
      tab: "engine",
      render: () => <p>Hello from my plugin</p>,
    });
  },
};

export default plugin;
```

### 3. Regenerate registry

```bash
bun run plugins:generate
```

This writes `src/lib/plugins/registry.generated.ts` (gitignored). Run after adding or updating plugins.

## Plugin API

| Method                        | Purpose                                                           |
| ----------------------------- | ----------------------------------------------------------------- |
| `ctx.addBrowseSite(site)`     | Add a site card on Browse (uses existing browse/download APIs)    |
| `ctx.addSettingsPanel(panel)` | Add a section under Settings (engine/general/lan/cookies/library) |
| `ctx.addNavItem(item)`        | Add a sidebar link                                                |

## Example

See [`plugins/example-plugin/`](../plugins/example-plugin/).

## Limitations (v1)

- Build-time discovery only (restart dev server after `plugins:generate`)
- No hot-load from arbitrary paths at runtime
- No Rust/native code in plugins
- No signed marketplace

## Hooks (future)

- Custom browse result transformers
- Download queue hooks
- YAML-defined tube sites without Rust
