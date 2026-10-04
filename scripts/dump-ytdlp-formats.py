"""Print yt-dlp format list for a URL (one line per format).

PowerShell-safe alternative to piping `yt-dlp -J` into `python -c`
(which breaks on quoting). Usage:

    python scripts/dump-ytdlp-formats.py https://de.chaturbate.com/pinkypuppa/
    python scripts/dump-ytdlp-formats.py --cookies cookies.txt <url>
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="Room/page URL to inspect")
    parser.add_argument(
        "--cookies", default=None, help="Netscape cookie file for yt-dlp"
    )
    parser.add_argument(
        "--ytdlp",
        default="yt-dlp",
        help="yt-dlp binary to invoke (default: yt-dlp)",
    )
    args = parser.parse_args()

    cmd = [
        args.ytdlp,
        "--skip-download",
        "-J",
        "--no-warnings",
        "--no-playlist",
        args.url,
    ]
    if args.cookies:
        cmd += ["--cookies", args.cookies]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    except FileNotFoundError:
        print(f"error: {args.ytdlp} binary not found", file=sys.stderr)
        return 2
    if proc.returncode != 0:
        print(proc.stderr.strip()[-2000:], file=sys.stderr)
        return 1
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        print(f"error: could not parse yt-dlp JSON: {e}", file=sys.stderr)
        return 1

    formats = data.get("formats") or []
    print(f"title: {data.get('title')}")
    print(f"is_live: {data.get('is_live')}")
    print(f"formats: {len(formats)}")
    for f in formats:
        url = str(f.get("url") or "")
        print(
            f"{f.get('format_id')} "
            f"v={f.get('vcodec')} a={f.get('acodec')} "
            f"proto={f.get('protocol')} ext={f.get('ext')} "
            f"{url[:130]}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
