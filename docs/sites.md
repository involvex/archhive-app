# Supported Sites

## Tier A — yt-dlp (browse + download)

| Site ID   | Display Name | Browse kinds                                 | Cookies                                                        |
| --------- | ------------ | -------------------------------------------- | -------------------------------------------------------------- |
| youtube   | YouTube      | channel, search, video                       | Optional                                                       |
| tiktok    | TikTok       | channel, search, video                       | No                                                             |
| twitter   | Twitter / X  | channel, video                               | Recommended                                                    |
| instagram | Instagram    | channel, video                               | Yes                                                            |
| thisvid   | ThisVid      | tag, search, video                           | Yes                                                            |
| thothub   | ThotHub      | tag, model, search, video                    | Optional (`thethothub.com`)                                    |
| pornhub   | PornHub      | category, tag, search, channel, model, video | Yes                                                            |
| youporn   | YouPorn      | category, model, search, video               | Optional                                                       |
| xnxx      | XNXX         | search, tag, channel, video                  | Optional                                                       |
| xhamster  | xHamster     | tag, search, channel, video                  | Yes                                                            |
| xvideos   | XVIDEOS      | tag, search, channel, video                  | Yes                                                            |
| reddit    | Reddit       | channel, search, video                       | Login cookies (Reddit hides listings from logged-out visitors) |
| eporner   | Eporner      | search, tag, category, video                 | No                                                             |

## Aggregators

| Site ID | Display Name | Browse kinds            | Cookies |
| ------- | ------------ | ----------------------- | ------- |
| xpomelo | xPomelo      | search, category, model | No      |

xPomelo hosts nothing: each result links to its source tube (PornHub,
XVIDEOS, xHamster, XNXX, Eporner, SpankBang, …). Items keep the source URL
and, where ArcHive has an adapter, the source site's id, so playback,
downloads and the Info dialog run through that site's own pipeline; other
sources go through plain yt-dlp. Category listings take any number of
required and excluded categories (`/category/a%2Cb%2C-c`). Results from
paysites (Brazzers, FapHouse, …) are hidden unless the Source filter is set.
Cloudflare blocks HTTP/1.1 clients on this site; the main HTTP/2 client passes.

### Bot checks (Cloudflare) on Android

Some sources (SpankBang, KVS tubes such as CrazyPorn, Love4Porn, 4KPorn)
answer the embedded engines with a Cloudflare challenge. When playback or a
download fails that way on Android, ArcHive opens the page in its in-app
browser, where the check passes by itself or after one tap. It then reads
the video URL from the rendered page (`sites/challenge.rs`) and follows
session-bound KVS `get_file` links inside the same browser session. Sites
with dedicated adapters (PornHub, XVIDEOS, …) are excluded.

## Tier B — Live Cam Sites (scraping + webview bridge)

| Site ID    | Display Name | Browse kinds                   | Cookies |
| ---------- | ------------ | ------------------------------ | ------- |
| chaturbate | Chaturbate   | livestream, tag, search, model | Yes     |
| stripchat  | Stripchat    | livestream, tag, search, model | Yes     |

## Tier C — gallery-dl

| Site ID | Display Name | Notes                                                                |
| ------- | ------------ | -------------------------------------------------------------------- |
| redgifs | RedGifs      | Browse via the RedGifs v2 API (temporary token); direct MP4 download |

## Cookie Requirements

Sites marked **Yes** for cookies typically need exported browser cookies for full access. See [SCrawler Settings wiki](https://github.com/AAndyProgram/SCrawler/wiki/Settings) for per-site guidance.

## Tools

- **ffmpeg** — remux/merge for some sites (install separately)
- **yt-dlp** — primary video downloader (must be on PATH)
- **gallery-dl** — RedGifs and gallery content

## Metadata Enrichment (StashDB)

Library scenes and performers can be enriched from [StashDB](https://stashdb.org)
via any configured stash-box endpoint (`https://stashdb.org/graphql` is
pre-seeded — setup is just paste-key). Works on desktop and Android
(pure `reqwest` + rustls, no Python); Remote LAN mode proxies through the
desktop host.

### Setup

1. **Settings → Metadata** — the StashDB endpoint is already listed.
2. Paste a StashDB API key (generate at `stashdb.org → Settings → API Key`)
   and **Test** — expect "Connected as …".
3. Keys are stored in the encrypted vault (`vault_secrets`), never in
   settings JSON or backups.

### Flows

- **Per-scene Enrich** — Scene Details → _Enrich from StashDB_. Matches by
  file fingerprint (full-file MD5 + OpenSubtitlesHash, sent with explicit
  algorithms); title search is offered only when no fingerprint matches.
  The title box is pre-cleaned (`STUDIO - Watch …` → lookup title) but stays
  editable. Pick a candidate, toggle which fields to merge
  (title/date/studio/performers/tags; cover download defaults **off** to
  spare mobile data), Apply. Enriched scenes show a StashDB badge, and
  fingerprints can be submitted back to StashDB from the same dialog.
- **Batch Identify** — Library → Scenes shows an Identify banner with the
  count of enrichable scenes (have MD5/OSHASH, no `stash_id` yet). Files
  hashed before the fingerprint fix show a separate amber banner with a
  **Compute hashes** button (one-shot MD5 + OSHASH repair; new files are
  hashed at import time). Matching is **fingerprint-only** (no title
  fallback) to avoid mismatches; verify per row (Apply / Apply all), cancel
  anytime. Our thumbnail pHash is _not_ sent — Stash uses an incompatible
  int64 pHash format, so only MD5/OSHASH go over the wire.
- **Performer linking** — Performer page → _Search StashDB_ → Apply links
  the StashDB id, merges aliases (deduped, capped), and sets the portrait
  only when none is stored.

## Sorting, filters and categories

What each site can strictly combine (tags, categories, search + category,
exclusions) is documented in [tag-combining.md](tag-combining.md).

Adapters advertise what they support via `SiteAdapter::browse_options()`
(returned in `SiteInfo.browse_options`); the browse page shows matching
Sort / Date / Length / Quality selects and passes the chosen ids as
`BrowseQuery.filters`. Unknown ids are dropped before they reach a URL.
Every page also has on-device filters (words, `-exclude`, length) over
loaded results. Category / tag indexes come from `list_categories()`
(`list_site_categories` command, `GET /api/sites/{id}/categories`).

| Site       | Sort                                    | Date / Length / Quality         | Categories            |
| ---------- | --------------------------------------- | ------------------------------- | --------------------- |
| PornHub    | newest, views, rating, longest, hottest | period, duration, HD            | JSON-embedded catalog |
| XVIDEOS    | relevance, date, rating, views, length  | period, duration, HD/1080p      | `/tags` (top 400)     |
| XNXX       | views, random                           | period, duration, HD/Full HD    | `/tags` (top 400)     |
| xHamster   | search: 5 sorts; category: newest/best  | duration, 720p/1080p/4K         | `/categories`         |
| YouPorn    | relevance, date, views, rating, length  | —                               | `/categories/`        |
| Eporner    | 7 API `order` values                    | —                               | `/cats/`              |
| xPomelo    | relevance, date, views, length          | period, duration, HD/4K, source | `data-tags` JSON (6k) |
| ThotHub    | KVS `sort_by` (5)                       | —                               | —                     |
| ThisVid    | KVS `sort_by` (5)                       | —                               | `/categories/`        |
| Reddit     | hot, new, top, rising                   | period (top)                    | —                     |
| RedGifs    | trending, latest, top                   | —                               | —                     |
| Chaturbate | —                                       | —                               | top hashtags API      |

## SCrawler Parity Roadmap

Future adapters: Threads, Pinterest, OnlyFans (cookie + DRM limitations), Bluesky, LPSG.

Port site logic from SCrawler as reference; do not bundle the .NET SCrawler binary.
