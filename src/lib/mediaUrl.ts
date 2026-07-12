import { convertFileSrc } from "@tauri-apps/api/core";
import { getAppRuntime, shouldUseRemoteApi } from "./runtime";
import { useSettingsStore } from "./stores/settings";
import type { Scene } from "./types";

const VIDEO_EXT = /\.(mp4|m4v|webm|mkv|avi|mov|wmv|flv)$/i;

function remoteMediaUrl(path: string, tokenQs: string): string | undefined {
  const { settings } = useSettingsStore.getState();
  const base = settings.remote_host?.replace(/\/$/, "");
  if (!base) return undefined;
  return `${base}${path}${tokenQs}`;
}

function authQuery(): string {
  const token = useSettingsStore.getState().settings.remote_token?.trim();
  return token ? `?token=${encodeURIComponent(token)}` : "";
}

/** Resolve a library scene thumbnail for desktop Tauri or remote LAN. */
export function sceneThumbUrl(scene: Scene): string | undefined {
  if (!scene.thumb && !scene.path) return undefined;

  if (shouldUseRemoteApi()) {
    return remoteMediaUrl(`/api/scenes/${scene.id}/thumb`, authQuery());
  }

  if (getAppRuntime() === "desktop-tauri" && scene.thumb) {
    return convertFileSrc(scene.thumb);
  }

  return scene.thumb;
}

/** Resolve playable media URL for a library scene (video/audio). */
export function sceneMediaUrl(scene: Scene): string | undefined {
  if (!scene.path) return undefined;

  if (shouldUseRemoteApi()) {
    return remoteMediaUrl(`/api/scenes/${scene.id}/media`, authQuery());
  }

  if (getAppRuntime() === "desktop-tauri") {
    return convertFileSrc(scene.path);
  }

  return undefined;
}

export function isVideoScene(scene: Scene): boolean {
  if (!scene.path) return false;
  return VIDEO_EXT.test(scene.path);
}

/** Stream URL for a file path relative to library root (folder browser). */
export function fileStreamUrl(relativePath: string): string | undefined {
  const { settings } = useSettingsStore.getState();
  const token = settings.remote_token?.trim() ?? settings.lan_token?.trim();

  if (shouldUseRemoteApi()) {
    const base = settings.remote_host?.replace(/\/$/, "");
    if (!base) return undefined;
    const pathParam = encodeURIComponent(relativePath.replace(/\\/g, "/"));
    const tokenPart = token ? `&token=${encodeURIComponent(token)}` : "";
    return `${base}/api/files/stream?path=${pathParam}${tokenPart}`;
  }

  if (getAppRuntime() === "desktop-tauri" && settings.lan_enabled) {
    const pathParam = encodeURIComponent(relativePath.replace(/\\/g, "/"));
    const tokenPart = token ? `&token=${encodeURIComponent(token)}` : "";
    return `http://127.0.0.1:${settings.lan_port}/api/files/stream?path=${pathParam}${tokenPart}`;
  }

  return undefined;
}

export function isVideoFilePath(name: string): boolean {
  return VIDEO_EXT.test(name);
}

export function isImageFilePath(name: string): boolean {
  return /\.(jpg|jpeg|png|webp|gif)$/i.test(name);
}
