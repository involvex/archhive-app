import type { BinaryVersions } from "./types";

/**
 * Android engine health derived from `binary_versions` (#48). The backend
 * queries the youtubedl-android Kotlin plugin directly, so the version
 * payload doubles as a plugin-registration signal — no extra IPC needed:
 * - `missing`: nothing reported → the YtDlp overlay/plugin isn't registered
 *   (stale `gen/android`, release R8 strip, skipped overlay).
 * - `degraded`: engine answers but media tools aren't unpacked yet.
 * - `ready`: yt-dlp + ffmpeg + ffprobe all report versions.
 */
export type EnginePluginStatus = "ready" | "degraded" | "missing";

export function enginePluginStatus(v: BinaryVersions | null): EnginePluginStatus {
  if (!v) return "missing";
  const has = (s?: string) => Boolean(s?.trim());
  const engine = has(v.ytdlp_version);
  const media = has(v.ffmpeg_version) && has(v.ffprobe_version);
  if (engine && media) return "ready";
  if (engine || has(v.ffmpeg_version) || has(v.ffprobe_version)) return "degraded";
  return "missing";
}
