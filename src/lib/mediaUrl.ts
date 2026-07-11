import { convertFileSrc } from "@tauri-apps/api/core";
import { getAppRuntime, shouldUseRemoteApi } from "./runtime";
import { useSettingsStore } from "./stores/settings";
import type { Scene } from "./types";

/** Resolve a library scene thumbnail for desktop Tauri or remote LAN. */
export function sceneThumbUrl(scene: Scene): string | undefined {
  if (!scene.thumb && !scene.path) return undefined;

  if (shouldUseRemoteApi()) {
    const { settings } = useSettingsStore.getState();
    const base = settings.remote_host?.replace(/\/$/, "");
    if (!base) return undefined;
    const token = settings.remote_token?.trim();
    const qs = token ? `?token=${encodeURIComponent(token)}` : "";
    return `${base}/api/scenes/${scene.id}/thumb${qs}`;
  }

  if (getAppRuntime() === "desktop-tauri" && scene.thumb) {
    return convertFileSrc(scene.thumb);
  }

  return scene.thumb;
}
