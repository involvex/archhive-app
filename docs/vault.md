# Media Vault

ArcHive's vault **moves** selected scenes into app-private storage and hides them
from the library while the session is locked.

## What it does

| Action           | Result                                                                                                                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Move to vault    | The video file **and** its `{stem}.jpg` thumbnail sidecar are moved into `<app data>/vault_media/`. The scene row stays in SQLite, flagged `is_vaulted = 1`, so ratings, tags, performers and watch history survive. |
| Lock / auto-lock | Every read path (listings, media, thumbnails, LAN endpoints) refuses vault content; the vault UI shows a lock screen.                                                                                                |
| Unlock           | Biometric prompt (`@tauri-apps/plugin-biometric`) → backend opens a session window for `vault_auto_lock_mins` minutes.                                                                                               |
| Restore          | Files move back to their original folder (or the library root if it disappeared) and the flags are cleared.                                                                                                          |

Bulk moves use the same path: select scenes in **Library → Scenes** (desktop
"Select multiple") and press **Move to vault** in the bulk bar. The vault unlocks
once up front and the per-scene failures are reported as a count.

Once the vault is enabled, **Library → Vault** is where locked content lives. It
always starts locked: opening the page only refreshes the backend status, and the
first unlock asks for the OS biometric/credential prompt.

## What it does **not** do

- **No encryption.** Vaulted bytes are plain files. A rooted device, a full
  filesystem backup, or adb with app-data access can read them. Encryption at
  rest is a possible follow-up (Android Keystore-wrapped key); it is not in v1.
- **No separate PIN.** The lock is a session lock plus an OS biometric/device
  credential prompt. The Rust side does not verify a secret; it trusts that the
  frontend ran the prompt. That is the v1 trust boundary: a modified frontend
  that calls `vault_unlock` directly skips the biometric check. Harden with a
  Keystore-backed gate before treating the vault as a security boundary against
  a determined local attacker.
- **No portable vault.** Vaulted files are tied to this app install (they live
  in app data). Uninstalling the app deletes them.
- **Desktop lock is UI-level only.** On phones the biometric prompt is an OS
  gate and vault media is served by the loopback HTTP server, which checks the
  lock. On desktop the webview can read local files through Tauri's asset
  protocol, so the desktop lock hides the vault from the UI (and from the LAN
  listener) but does not stop a modified frontend from reading the files.

## Why files disappear from gallery apps

On Android, `<app data>` is private internal storage: other apps cannot read it,
and `MediaStore` never indexes it. A `.nomedia` marker is written into
`vault_media/` as well, which covers platforms where the data directory happens
to sit on shared storage.

Vaulting also _moves_ a file that previously lived in a shared folder (say
`/storage/emulated/0/Download/ArcHive`). The move removes it from the shared
volume, so the gallery's next scan drops it. On Android ArcHive:

1. Retries filesystem delete after the verified copy
2. Matches MediaStore rows by `DISPLAY_NAME` / `RELATIVE_PATH` / size (not only
   the unreliable `DATA` column on scoped storage)
3. Calls `MediaStore.createDeleteRequest` on API 30+ when user consent is
   required, waiting for the system dialog result
4. Rescans + deletes again after short delays
5. Also vaults a conventional `{stem}.jpg` sidecar when `scene.thumb` is empty

If the original still cannot be removed, the vault IPC returns
`pending_delete_consent` / `leftover_paths` and the UI shows a warning toast.

## Settings

Settings → **Privacy**:

- **Enable the vault** — makes the Vault tab and "Move to vault" available.
- **Auto-lock after (minutes)** — `0` keeps the session open until you lock it or
  quit.
- **Lock when ArcHive goes to the background** — locks on `visibilitychange`
  (ignored while the biometric sheet is open).
- **Expose vaulted items over the LAN server** — off by default. When off, LAN
  clients get `404` for vaulted scenes' media/metadata even while the desktop
  session is unlocked, and `/api/files` never lists the vault directory. A
  bearer token is not a biometric check.

Even when exposure is on, LAN **listings** are still filtered in SQL: a remote
client can fetch a vaulted scene by id, but vaulted rows never show up in
`/api/scenes` or search results.

## Database

`MIGRATION_021` adds:

| Column                | Meaning                                       |
| --------------------- | --------------------------------------------- |
| `is_vaulted`          | `1` while the scene is in the vault           |
| `vault_original_path` | Pre-vault location, used for an exact restore |
| `vaulted_at`          | RFC 3339 timestamp                            |

`scenes.path` and `scenes.thumb` point into the vault while vaulted; every read
must go through `AppState::scene_media_path`, which re-checks the lock.

Vaulted rows are also excluded from the batch maintenance queries (missing
thumbnails, duration probes, hash backfill, StashDB enrichment, duplicate
grouping) so a locked vault is not touched or surfaced by background work.
`Database::get_scene` still returns a vaulted row — it is the vault's own
plumbing — so the IPC layer gates it: `get_scene`, `open_scene_in_explorer` and
`open_scene_with_default` answer "Scene not found" while the vault is closed
(`ensure_scene_visible` in `commands.rs`), and `server/mod.rs` answers `404`.

## Files

- `src-tauri/src/secure_vault/mod.rs` — `MediaVault` (lock, move, restore)
- `src-tauri/src/state.rs` — `vault_status`, `unlock_vault`, `vault_scene(s)`,
  `unvault_scene`, `list_vaulted_scenes`, `scene_media_path`
- `src-tauri/src/commands.rs` — `vault_*` IPC commands
- `src-tauri/src/server/mod.rs` — `vault_media_allowed` / `guard_vault_path`
- `src/lib/stores/vault.ts`, `src/routes/library/vault.tsx` — UI
- `src/lib/biometrics.ts` — biometric prompt wrapper

## Rescan safety

`LibraryScanner::scan` skips any directory that is dot-prefixed or contains a
`.nomedia` marker, so a vault nested under the library path is never re-imported,
even after the vault is locked again.
