# ArcHive — AI Agent Instructions

Cross-platform media browser, downloader, and personal library built with **Tauri v2 + React 19 + Rust**. Use this document as the authoritative reference when working in this codebase.

---

## Project Overview

ArcHive is a desktop and mobile application that lets users browse, download, and organize media from multiple sites (YouTube, TikTok, Reddit, PornHub, xHamster, XVIDEOS, RedGifs, ThotHub, and more). It uses **yt-dlp** and **gallery-dl** as download engines, stores a local library in **SQLite**, and exposes an optional **LAN REST API** for mobile clients.

**Key files to read first:**

- `Plan.md` — full architecture, site adapter docs, library model
- `README.md` — quick start, scripts, feature overview
- `docs/sites.md` — supported sites, cookie requirements, adapter tiers
- `docs/custom-sites.md` — how to add new site adapters
- `docs/mobile-android.md` — Android dev setup, Remote LAN testing
- `docs/cookie-import.md` — cookie vault import workflow

---

## Useful Commands

### Development

```bash
bun install                  # install JS dependencies
bun run dev                  # Vite dev server (localhost:1420)
bun run tauri dev            # Desktop app with hot reload
bun run tauri:android:dev    # Android app on device/emulator
```

### Build & Production

```bash
bun run build                # TypeScript check + Vite production build
bun run tauri build          # Full Tauri desktop bundle
bun run tauri android build  # Release APK/AAB
```

### Code Quality

```bash
bun run lint                 # ESLint (TypeScript + React rules)
bun run format               # Prettier write (all files)
bun run format:check         # Prettier check (CI-friendly)
```

### Rust Backend

```bash
cd src-tauri && cargo test   # Run Rust unit/integration tests
cd src-tauri && cargo clippy # Lint Rust code
cd src-tauri && cargo fmt    # Format Rust code
```

### Binary Setup

```bash
bun run setup:binaries       # Download yt-dlp + ffmpeg sidecars
# Windows: .\scripts\setup-binaries.ps1
# macOS/Linux: ./scripts/setup-binaries.sh
```

### Android

```bash
bun run tauri android init   # One-time Android project setup
.\scripts\patch-android-lan.ps1  # Enable cleartext HTTP for LAN (release)
```

---

## Tech Stack

| Layer               | Technology                                  | Notes                                                |
| ------------------- | ------------------------------------------- | ---------------------------------------------------- |
| **Shell**           | Tauri v2                                    | Desktop (Windows/macOS/Linux) + mobile (iOS/Android) |
| **Backend**         | Rust 2021 edition                           | IPC commands, downloads, DB, LAN server              |
| **Frontend**        | React 19 + TypeScript 5.8                   | Strict mode enabled                                  |
| **Routing**         | TanStack Router                             | File-based routes in `src/routes/`                   |
| **Styling**         | Tailwind CSS v4                             | Dark-first, oklch color tokens                       |
| **Components**      | Radix UI primitives + shadcn-style wrappers | `src/components/ui/`                                 |
| **State**           | Zustand 5                                   | Global stores in `src/lib/stores/`                   |
| **Package manager** | Bun (>=1.3.0)                               | Do not use npm/yarn/pnpm                             |
| **Build**           | Vite 7                                      | With React + TanStack Router plugins                 |
| **Database**        | SQLite (rusqlite, bundled) + FTS5           | Embedded in app data dir                             |
| **HTTP**            | reqwest 0.12                                | For site scraping and yt-dlp integration             |
| **HTML parsing**    | scraper 0.22                                | CSS selector-based DOM parsing                       |
| **LAN server**      | Axum 0.8 + tower-http                       | CORS, static file serving, mDNS                      |
| **Encryption**      | AES-256-GCM (aes-gcm)                       | Cookie vault at rest                                 |
| **Perceptual hash** | image_hasher                                | Duplicate detection                                  |

### Rust Dependencies (key crates)

| Crate                         | Purpose                                      |
| ----------------------------- | -------------------------------------------- |
| `tauri`                       | App shell, IPC, window management            |
| `serde` / `serde_json`        | Serialization for IPC and API                |
| `tokio`                       | Async runtime (full features)                |
| `rusqlite`                    | SQLite with bundled driver                   |
| `reqwest`                     | HTTP client for scraping                     |
| `scraper`                     | HTML parsing                                 |
| `axum`                        | LAN REST API server                          |
| `tower-http`                  | CORS, static files, tracing                  |
| `mdns-sd`                     | LAN service discovery                        |
| `thiserror` / `anyhow`        | Error handling                               |
| `async-trait`                 | Async trait methods (site adapters)          |
| `aes-gcm` + `base64` + `sha2` | Cookie vault encryption                      |
| `walkdir`                     | Recursive directory traversal (library scan) |
| `image` + `image_hasher`      | Thumbnail processing, pHash                  |

---

## Project Structure

```
scrawler-app/
├── src/                        # React frontend
│   ├── routes/                 # TanStack file-based routes
│   │   ├── __root.tsx          # Root layout (AppShell wrapper)
│   │   ├── index.tsx           # Home page
│   │   ├── browse/             # Site browsing
│   │   ├── downloads/          # Download queue
│   │   ├── library/            # Local library (scenes, performers, tags)
│   │   └── settings/           # App settings, cookies, LAN
│   ├── components/             # Shared UI components
│   │   ├── ui/                 # Base UI (button, card, input, progress)
│   │   ├── AppShell.tsx        # Main layout shell
│   │   ├── SceneCard.tsx       # Scene display card
│   │   ├── DownloadProgress.tsx# Download progress indicator
│   │   └── DuplicateGroupCard.tsx
│   ├── lib/
│   │   ├── api/client.ts       # IPC + LAN client abstraction
│   │   ├── stores/             # Zustand stores (downloads, settings)
│   │   ├── cookies/            # Cookie import utilities
│   │   ├── types.ts            # Shared TypeScript types
│   │   └── utils.ts            # cn() helper (clsx + tailwind-merge)
│   ├── styles/globals.css      # Tailwind + theme tokens
│   └── main.tsx                # App entry point
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── lib.rs              # Tauri builder + plugin setup
│   │   ├── main.rs             # Desktop entry point
│   │   ├── commands.rs         # All #[tauri::command] handlers
│   │   ├── models.rs           # Shared data structures
│   │   ├── state.rs            # AppState (central coordinator)
│   │   ├── error.rs            # AppError enum + AppResult type
│   │   ├── db/                 # SQLite schema, queries, migrations
│   │   ├── downloads/          # Download manager + queue
│   │   ├── sites/              # Site adapter registry + adapters
│   │   │   └── adapters/       # Per-site adapter implementations
│   │   ├── library/            # Library scanner + auto-tagger
│   │   ├── server/             # Axum LAN server + mDNS
│   │   ├── vault/              # Encrypted cookie vault
│   │   ├── media/              # Media processing
│   │   └── mobile/             # Mobile-specific code (standalone mode)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── capabilities/           # Tauri v2 capability permissions
├── docs/                       # Documentation
├── scripts/                    # Setup and utility scripts
├── public/                     # Static assets
├── Plan.md                     # Architecture plan
├── package.json
├── vite.config.ts
├── tsconfig.json
├── eslint.config.js
└── .prettierrc
```

---

## Architecture Patterns

### Frontend (React/TypeScript)

- **File-based routing**: Routes live in `src/routes/`. The route tree is auto-generated by `@tanstack/router-plugin/vite` into `src/routeTree.gen.ts`. Never edit `routeTree.gen.ts` manually.
- **IPC abstraction**: All backend calls go through `src/lib/api/client.ts`. The `api` object provides typed methods that transparently switch between local Tauri `invoke()` and remote LAN `fetch()` based on the current engine mode.
- **State management**: Zustand stores in `src/lib/stores/` for global state (downloads, settings). Keep stores minimal and focused.
- **Component organization**: Shared UI primitives in `src/components/ui/` (shadcn-style). Feature components directly in `src/components/`.
- **Styling**: Tailwind CSS v4 with custom theme tokens in `globals.css`. Use `cn()` from `src/lib/utils.ts` for conditional classes.
- **Type safety**: TypeScript strict mode is enabled. All types shared with the Rust backend are defined in `src/lib/types.ts` and must stay in sync with `src-tauri/src/models.rs`.

### Backend (Rust/Tauri)

- **Command pattern**: All IPC endpoints are `#[tauri::command]` functions in `commands.rs`. They delegate to `AppState` methods which coordinate between subsystems.
- **State management**: `AppState` (in `state.rs`) holds `Arc`-wrapped references to Database, SiteRegistry, DownloadManager, CookieVault, and LanServer.
- **Error handling**: Use `AppError` enum (in `error.rs`) with `thiserror`. Convert to `String` at the command boundary via `map_err()`.
- **Site adapters**: Implement the `SiteAdapter` trait (`async_trait`) for each site. Register in `SiteRegistry`. See `docs/custom-sites.md` for the full pattern.
- **Database**: SQLite with rusqlite. Schema and migrations in `src-tauri/src/db/`. Uses FTS5 for full-text search.
- **LAN server**: Axum-based REST API in `src-tauri/src/server/`. Bearer token auth. Advertises via mDNS (`_archhive._tcp`).

### Data Flow

```
User Action → React Component → api.client.ts
  ├─ Local mode → Tauri invoke("command_name") → commands.rs → AppState → subsystem
  └─ Remote LAN mode → fetch("http://desktop:8787/api/...") → Axum handlers → AppState
```

---

## Best Practices & Guidelines

### General

1. **Never hardcode secrets, tokens, or API keys**. The cookie vault uses AES-256-GCM encryption. LAN tokens are generated randomly.
2. **Keep TypeScript types and Rust models in sync**. `src/lib/types.ts` and `src-tauri/src/models.rs` must mirror each other. When adding a new model, update both files.
3. **Use the `@/` path alias** for all imports from `src/` (e.g., `import { api } from "@/lib/api/client"`). The alias is configured in `tsconfig.json` and `vite.config.ts`.
4. **Run lint and format before committing**:
   ```bash
   bun run lint && bun run format:check
   cd src-tauri && cargo clippy && cargo fmt --check
   ```

### Frontend (React/TypeScript)

5. **Strict TypeScript**: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` are all enabled. Fix all errors before committing.
6. **React hooks rules**: ESLint enforces `react-hooks` rules. Do not disable them.
7. **Prefer composition over prop drilling**. Use Zustand stores for shared state, React context for local scope.
8. **Route components are lazy-loaded** by default via TanStack Router. Do not add manual `React.lazy()` unless necessary.
9. **UI components**: Extend or create components in `src/components/ui/` using Radix primitives. Follow the shadcn pattern: accept `className`, merge with `cn()`, forward refs.
10. **Dark-first design**: The app defaults to dark mode (`class="dark"` on `<html>`). Always design for dark backgrounds first. Use oklch color tokens from the theme.

### Backend (Rust)

11. **Async everywhere**: All network and IO operations must be async (tokio). Use `block_on` only in Tauri command handlers when needed.
12. **Error propagation**: Use `?` operator with `AppError`. Never use `.unwrap()` in production code paths. Use `.ok_or()` or `.ok_or_else()` with descriptive messages.
13. **Thread safety**: Use `Arc<T>` for shared state, `Mutex`/`parking_lot::Mutex` for interior mutability. Never hold locks across `.await` points.
14. **Serde serialization**: Use `#[serde(rename_all = "snake_case")]` on enums and structs that cross the IPC boundary. This matches the frontend's camelCase convention via Tauri's default serde rename.
15. **Command handlers**: Keep `commands.rs` thin — delegate to `AppState` methods. The command layer is only responsible for type conversion and error mapping.
16. **Testing**: Write unit tests in-module (`#[cfg(test)]` mod tests). Use `tempfile` for filesystem tests. Test site adapters with saved HTML fixtures.

### Site Adapters

17. **Implement `SiteAdapter` trait** from `src-tauri/src/sites/adapters/`. See existing adapters for patterns.
18. **Register new adapters** in `src-tauri/src/sites/registry.rs`.
19. **Download strategy**: Most sites resolve to yt-dlp. Use `DownloadTool::GalleryDl` for image galleries. Use `DownloadTool::DirectHttp` for direct media URLs.
20. **Cookie handling**: Access cookies via `SiteContext` and `CookieVault`. Never store cookies in plaintext outside the vault.

### Security

21. **Cookie vault**: Cookies are encrypted at rest with AES-256-GCM. Plaintext cookie files are written only temporarily for yt-dlp `--cookies` usage.
22. **LAN server**: Requires bearer token for all endpoints except `/api/health`. Tokens are randomly generated 32-byte hex strings.
23. **CSP**: Currently disabled (`"csp": null`) in `tauri.conf.json` for development flexibility. Re-enable before production release.
24. **No user input in shell commands**: When invoking yt-dlp or gallery-dl, sanitize URLs and never interpolate untrusted input into shell commands. Use `Command::new()` with proper argument arrays.

### Performance

25. **Lazy loading**: Route components are code-split by TanStack Router. Do not import heavy dependencies at the top level.
26. **Database queries**: Use FTS5 for search queries. Add indexes for frequently filtered columns.
27. **Image processing**: Use `image` crate for thumbnails. Perceptual hashing (`image_hasher`) for duplicate detection runs on a configurable threshold (default pHash distance: 10).
28. **Vite dev server**: Runs on port 1420 with strict port. HMR on port 1421 when using remote dev host. The `src-tauri/` directory is excluded from file watching.

### File Operations

29. **Windows paths**: Use backslashes in file paths but forward slashes work in most tools. When cross-platform compatibility matters, use `path::PathBuf` (Rust) or `path` module (Node).
30. **Binary sidecars**: Bundled binaries (`yt-dlp`, `ffmpeg`) live in `src-tauri/binaries/`. The `.gitignore` excludes them except for `.gitkeep` and `README.md`. Run `bun run setup:binaries` to download.
31. **Generated files**: Never edit `src/routeTree.gen.ts` manually. It is auto-generated by the TanStack Router Vite plugin.

---

## Runtime Modes

| Mode           | Platform         | Capability                        |
| -------------- | ---------------- | --------------------------------- |
| **Local**      | Desktop          | Full yt-dlp / gallery-dl / ffmpeg |
| **Standalone** | Mobile           | YouTube + direct media URLs only  |
| **Remote LAN** | Mobile → desktop | Full parity via REST API          |

Configure in **Settings → Engine**. The frontend client (`api/client.ts`) automatically switches between local IPC and remote HTTP based on this setting.

---

## Testing

```bash
# Rust tests
cd src-tauri && cargo test

# Frontend lint
bun run lint

# Format check
bun run format:check
```

- Rust tests use `tempfile` for filesystem isolation.
- Site adapter tests should use saved HTML fixtures in `src-tauri/tests/fixtures/`.
- There is no frontend test runner configured yet. When adding tests, prefer Vitest for consistency with Vite.

---

## Common Tasks

### Adding a new site adapter

1. Create `src-tauri/src/sites/adapters/mysite.rs`
2. Implement `SiteAdapter` trait (see `docs/custom-sites.md`)
3. Register in `src-tauri/src/sites/registry.rs`
4. Add TypeScript types if needed in `src/lib/types.ts`
5. Update `docs/sites.md`

### Adding a new Tauri command

1. Define the function in `src-tauri/src/commands.rs` with `#[tauri::command]`
2. Register in `lib.rs` invoke handler: `commands::my_command`
3. Add the TypeScript wrapper in `src/lib/api/client.ts`
4. Add TypeScript types in `src/lib/types.ts` (mirror Rust model)

### Adding a new UI route

1. Create a file in `src/routes/myroute.tsx`
2. Export a `Route` using TanStack Router's `createFileRoute`
3. The route tree auto-registers on next `bun run dev`

### Modifying the database schema

1. Add migration SQL in `src-tauri/src/db/`
2. Update `Database` methods in the same module
3. Update or add Rust models in `models.rs`
4. Mirror changes in `src/lib/types.ts`
5. Add any new commands in `commands.rs`

---

## References

- [Tauri v2 Docs](https://v2.tauri.app/)
- [TanStack Router](https://tanstack.com/router)
- [Tailwind CSS v4](https://tailwindcss.com/)
- [Radix UI](https://www.radix-ui.com/)
- [Zustand](https://zustand-demo.pmnd.rs/)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [gallery-dl](https://github.com/mikf/gallery-dl)
- [SCrawler (reference)](https://github.com/AAndyProgram/SCrawler)
- [Stash (reference)](https://github.com/stashapp/stash)

---

## Learned User Preferences

- Do not create git commits unless the user explicitly asks
- Run `bun run lint`, `bun run format:check`, and `cd src-tauri && cargo test` before declaring work complete
- Keep the project on `I:\` — user is low on `C:\` storage; Android dev must work from the I: drive
- Enable the LAN server on the desktop ArcHive app, not on the mobile client
- Do not push to remote without explicit user permission

---

## Learned Workspace Facts

- Mobile Remote LAN connects to the desktop Axum API on port **8787**, not the Vite dev server port 1420
- Android builds do not bundle yt-dlp/ffmpeg sidecars; `externalBin` is desktop-only (`tauri.windows.conf.json`, `.macos`, `.linux`) and `tauri.android.conf.json` keeps an empty list
- `reqwest` uses **rustls** (not OpenSSL) to avoid NDK OpenSSL setup for Android cross-compiles
- Mobile defaults to `remote_lan` engine mode; browse/download/library APIs route over HTTP to the desktop LAN host
- Mobile bottom navigation must include **Settings** (Home, Browse, Downloads, Library, Settings)
- Windows Android helpers: `bun run android:dev` (auto-boot AVD + deploy) and `bun run android:regen` (regenerate `gen/android` after identifier changes)
- Repo on `I:\` with Cargo registry on `C:\` can trigger Kotlin "different roots" errors — `android:regen` sets `kotlin.incremental=false`
- Physical Android device: set Remote LAN host to `http://<pc-lan-ip>:8787`; Android emulator uses `http://10.0.2.2:8787`
- After changing `identifier` in `tauri.conf.json`, run `bun run android:regen` to fix stale `gen/android` package paths
