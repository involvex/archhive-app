import { convertFileSrc } from "@tauri-apps/api/core";
import { getAppRuntime, shouldUseRemoteApi } from "./runtime";
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

export function sceneThumbUrl(scene: Scene): string | undefined {
  // Only request thumbs when a real image path exists (never use video path).
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

  return scene.thumb;
}

/** Resolve playable media URL for a library scene (video/audio). */
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
