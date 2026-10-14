import { convertFileSrc } from "@tauri-apps/api/core";
import { getAppRuntime, hasLocalBackend, shouldUseRemoteApi } from "./runtime";
import { useSettingsStore } from "./stores/settings";
import type { Scene } from "./types";

const VIDEO_EXT = /\.(mp4|m4v|webm|mkv|avi|mov|wmv|flv)$/i;
const PLAYABLE_IN_WEBVIEW = /\.(mp4|m4v|webm)$/i;

/** Token for remote API clients (phone / browser → desktop). */
function remoteAuthToken(): string | undefined {
  const { settings } = useSettingsStore.getState();
  return settings.remote_token?.trim() || settings.lan_token?.trim() || undefined;
}

/** Token for desktop loopback LAN (must match the local server's lan_token). */
function lanAuthToken(): string | undefined {
  const { settings } = useSettingsStore.getState();
  return settings.lan_token?.trim() || undefined;
}

function tokenQuery(token: string | undefined): string {
  return token ? `?token=${encodeURIComponent(token)}` : "";
}

function tokenAmp(token: string | undefined): string {
  return token ? `&token=${encodeURIComponent(token)}` : "";
}

function remoteMediaUrl(path: string, tokenQs: string): string | undefined {
  const { settings } = useSettingsStore.getState();
  const base = settings.remote_host?.replace(/\/$/, "");
  if (!base) return undefined;
  return `${base}${path}${tokenQs}`;
}

/** Image hosts that refuse requests without their own site as Referer
 * (Pornwitch serves PornHub thumbnails from its CDN, 403 otherwise). */
const REFERER_GATED_HOSTS: { host: string; referer: string }[] = [
  { host: "cdn.pornwitch.com", referer: "https://pornwitch.com/" },
];

/**
 * A remote thumbnail URL the WebView can load: images on Referer-gated hosts
 * go through the app's media proxy (which sends the right Referer and caches
 * them); everything else is returned unchanged.
 */
export function listingThumbUrl(url: string | undefined): string | undefined {
  if (!url || !/^https?:\/\//.test(url)) return url;
  let host = "";
  try {
    host = new URL(url).hostname;
  } catch {
    return url;
  }
  const gated = REFERER_GATED_HOSTS.find((h) => host === h.host);
  if (!gated) return url;
  const qs = `?url=${encodeURIComponent(url)}&referer=${encodeURIComponent(gated.referer)}`;
  const { settings } = useSettingsStore.getState();
  if (shouldUseRemoteApi()) {
    return remoteMediaUrl(`/api/media/proxy${qs}`, tokenAmp(remoteAuthToken())) ?? url;
  }
  if (getAppRuntime() === "mobile-tauri" && hasLocalBackend()) {
    return `http://127.0.0.1:${settings.lan_port}/api/media/proxy${qs}`;
  }
  if (getAppRuntime() === "desktop-tauri" && settings.lan_enabled) {
    return `http://127.0.0.1:${settings.lan_port}/api/media/proxy${qs}${tokenAmp(lanAuthToken())}`;
  }
  return url;
}

export function sceneThumbUrl(scene: Scene): string | undefined {
  if (!scene.thumb) return undefined;

  const { settings } = useSettingsStore.getState();

  if (shouldUseRemoteApi()) {
    return remoteMediaUrl(`/api/scenes/${scene.id}/thumb`, tokenQuery(remoteAuthToken()));
  }

  if (getAppRuntime() === "desktop-tauri") {
    if (settings.lan_enabled) {
      return `http://127.0.0.1:${settings.lan_port}/api/scenes/${scene.id}/thumb${tokenQuery(lanAuthToken())}`;
    }
    return convertFileSrc(scene.thumb);
  }

  if (getAppRuntime() === "mobile-tauri" && hasLocalBackend()) {
    return `http://127.0.0.1:${settings.lan_port}/api/scenes/${scene.id}/thumb`;
  }

  return scene.thumb;
}

/** Resolve playable media URL for a library scene (video/audio).
 *  On the phone the 127.0.0.1 loopback server always runs, so it does not
 *  depend on `lan_enabled` (which can still read false while it starts). */
export function sceneMediaUrl(scene: Scene): string | undefined {
  if (!scene.path) return undefined;

  const { settings } = useSettingsStore.getState();

  if (shouldUseRemoteApi()) {
    return remoteMediaUrl(`/api/scenes/${scene.id}/media`, tokenQuery(remoteAuthToken()));
  }

  if (getAppRuntime() === "desktop-tauri") {
    if (settings.lan_enabled) {
      return `http://127.0.0.1:${settings.lan_port}/api/scenes/${scene.id}/media${tokenQuery(lanAuthToken())}`;
    }
    return convertFileSrc(scene.path);
  }

  if (getAppRuntime() === "mobile-tauri" && hasLocalBackend()) {
    return `http://127.0.0.1:${settings.lan_port}/api/scenes/${scene.id}/media`;
  }

  return undefined;
}

export function isVideoScene(scene: Scene): boolean {
  if (!scene.path) return false;
  return VIDEO_EXT.test(scene.path);
}

/** Formats the in-app WebView/LAN player can usually decode. */
export function isWebPlayableScene(scene: Scene): boolean {
  if (!scene.path) return false;
  return PLAYABLE_IN_WEBVIEW.test(scene.path);
}

/** Stream URL for a file path relative to library root (folder browser). */
export function fileStreamUrl(relativePath: string): string | undefined {
  const { settings } = useSettingsStore.getState();
  const pathParam = encodeURIComponent(relativePath.replace(/\\/g, "/"));

  if (shouldUseRemoteApi()) {
    const base = settings.remote_host?.replace(/\/$/, "");
    if (!base) return undefined;
    return `${base}/api/files/stream?path=${pathParam}${tokenAmp(remoteAuthToken())}`;
  }

  if (getAppRuntime() === "desktop-tauri" && settings.lan_enabled) {
    return `http://127.0.0.1:${settings.lan_port}/api/files/stream?path=${pathParam}${tokenAmp(lanAuthToken())}`;
  }

  if (getAppRuntime() === "mobile-tauri" && hasLocalBackend()) {
    const root = settings.library_path;
    if (root) return `http://127.0.0.1:${settings.lan_port}/api/files/stream?path=${pathParam}`;
  }

  return undefined;
}

export function isVideoFilePath(name: string): boolean {
  return VIDEO_EXT.test(name);
}

export function isImageFilePath(name: string): boolean {
  return /\.(jpg|jpeg|png|webp|gif)$/i.test(name);
}

/** True when the media URL needs CORS credentials mode (HTTP(S) only). */
export function isHttpMediaSrc(src: string | undefined): boolean {
  if (!src) return false;
  return /^https?:\/\//i.test(src);
}

/**
 * Explain why a library file failed to play: probes the stream URL (one
 * byte) so a missing file or a path outside the library folder isn't
 * reported as a codec problem.
 */
export async function describePlaybackFailure(src: string | undefined): Promise<string> {
  if (!src) return "No playable URL for this file.";
  if (!/^https?:/i.test(src)) return "The in-app player could not play this file.";
  try {
    const res = await fetch(src, { headers: { Range: "bytes=0-0" } });
    if (res.status === 404) return "The file is missing on disk (moved or deleted?).";
    if (res.status === 403) {
      return "This file is outside the library folder, so the app will not stream it. Check Settings → Library.";
    }
    if (!res.ok) return `The media server returned HTTP ${res.status}.`;
    return "The file is reachable, but its video/audio format can't play in the in-app player.";
  } catch {
    return "The on-device media server is not reachable. Restart the app and try again.";
  }
}
