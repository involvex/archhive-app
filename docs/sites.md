# Supported Sites

## Tier A — yt-dlp (browse + download)

| Site ID  | Display Name | Browse kinds                | Cookies              |
| -------- | ------------ | --------------------------- | -------------------- |
| youtube  | YouTube      | channel, search, video      | Optional             |
| tiktok   | TikTok       | channel, video              | No                   |
| twitter  | Twitter / X  | channel, video              | Recommended          |
| thisvid  | ThisVid      | tag, search, video          | Yes                  |
| thothub  | ThotHub      | tag, model, search, video   | Optional             |
| pornhub  | PornHub      | tag, search, channel, video | Yes                  |
| xhamster | xHamster     | tag, search, channel, video | Yes                  |
| xvideos  | XVIDEOS      | tag, search, channel, video | Yes                  |
| reddit   | Reddit       | channel, search, video      | OAuth for subreddits |

## Tier B — gallery-dl

| Site ID | Display Name | Notes                        |
| ------- | ------------ | ---------------------------- |
| redgifs | RedGifs      | Uses gallery-dl for download |

## Cookie Requirements

Sites marked **Yes** for cookies typically need exported browser cookies for full access. See [SCrawler Settings wiki](https://github.com/AAndyProgram/SCrawler/wiki/Settings) for per-site guidance.

## Tools

- **ffmpeg** — remux/merge for some sites (install separately)
- **yt-dlp** — primary video downloader (must be on PATH)
- **gallery-dl** — RedGifs and gallery content

## SCrawler Parity Roadmap

Future adapters: Instagram, Threads, Pinterest, OnlyFans (cookie + DRM limitations), Bluesky, LPSG.

Port site logic from SCrawler as reference; do not bundle the .NET SCrawler binary.
