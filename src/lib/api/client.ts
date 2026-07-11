import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  BrowseKind,
  BrowseOrientation,
  BrowsePage,
  DownloadJob,
  DuplicateGroup,
  CookieSiteInfo,
  HealthResponse,
  LanHost,
  MediaItem,
  MergeDuplicatesResult,
  Performer,
  Scene,
  SiteInfo,
  Tag,
  UpdateSceneRequest,
  BatchUpdateScenesRequest,
  BatchUpdateScenesResult,
  PornhubCategoryEntry,
} from "../types";
import { getAppRuntime, shouldUseRemoteApi } from "../runtime";
import { useSettingsStore } from "../stores/settings";
import { isDesktopTauri } from "../tauri";

function getRemoteBase(): string | null {
  const { settings } = useSettingsStore.getState();
  const host = settings.remote_host?.replace(/\/$/, "");
  if (!host) return null;
  return host;
}

function remoteHeaders(includeToken = true): HeadersInit {
  const { settings } = useSettingsStore.getState();
  const headers: HeadersInit = { "Content-Type": "application/json" };
  const token = settings.remote_token?.trim();
  if (includeToken && token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  return headers;
}

async function remoteFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const base = getRemoteBase();
  if (!base) {
    throw new Error(
      "Remote host not configured. Open Settings → Engine and pick a discovered LAN host.",
    );
  }
  const doFetch = (headers: HeadersInit) =>
    fetch(`${base}${path}`, {
      ...init,
      headers: { ...headers, ...init?.headers },
    });

  let res: Response;
  try {
    res = await doFetch(remoteHeaders(true));
  } catch (e) {
    const host = base.replace(/^https?:\/\//, "");
    const hint =
      e instanceof TypeError
        ? `Cannot reach ${host}. Check PC IP, Windows Firewall (TCP 8787), same Wi‑Fi, and cleartext HTTP on Android.`
        : e instanceof Error
          ? e.message
          : "Network error";
    throw new Error(hint, e instanceof Error ? { cause: e } : undefined);
  }
  if (res.status === 401) {
    const text = await res.text();
    const hadToken = Boolean(useSettingsStore.getState().settings.remote_token?.trim());
    if (hadToken) {
      useSettingsStore.getState().updateSettings({ remote_token: undefined });
      res = await doFetch(remoteHeaders(false));
      if (res.ok) {
        return res.status === 204 ? (undefined as T) : ((await res.json()) as T);
      }
    }
    throw new Error(
      text.includes("invalid token")
        ? "Invalid token — cleared saved token. Retry without a token in Settings."
        : text || "Unauthorized",
    );
  }
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  if (res.status === 204) return undefined as T;
  const contentType = res.headers.get("content-type") ?? "";
  if (!contentType.includes("application/json")) {
    const text = await res.text();
    if (text.startsWith("<!")) {
      throw new Error(
        "Received HTML instead of JSON. Use port 8787 (LAN API), not 1420 (Vite dev UI).",
      );
    }
    throw new Error(text || "Invalid response from remote host");
  }
  return res.json() as Promise<T>;
}

async function localInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (getAppRuntime() !== "desktop-tauri") {
    throw new Error("This action requires the desktop app.");
  }
  return invoke<T>(command, args);
}

async function localOrRemote<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  remotePath: string,
  init?: RequestInit,
): Promise<T> {
  if (shouldUseRemoteApi()) {
    return remoteFetch<T>(remotePath, init);
  }
  return localInvoke<T>(command, args);
}

export interface ScanProgress {
  scanned: number;
  added: number;
  updated: number;
}

export const api = {
  async health(): Promise<HealthResponse> {
    return localOrRemote("health", undefined, "/api/health");
  },

  async listSites(): Promise<SiteInfo[]> {
    return localOrRemote("list_sites", undefined, "/api/sites");
  },

  async browse(
    siteId: string,
    kind: BrowseKind,
    slug: string,
    page = 1,
    orientation?: BrowseOrientation,
  ): Promise<BrowsePage> {
    const orientParam = orientation ? `&orientation=${orientation}` : "";
    return localOrRemote(
      "browse",
      { siteId, kind, slug, page, orientation },
      `/api/sites/${siteId}/browse?kind=${kind}&slug=${encodeURIComponent(slug)}&page=${page}${orientParam}`,
    );
  },

  async queueDownload(url: string, adapter?: string): Promise<DownloadJob> {
    return localOrRemote("queue_download", { url, adapter }, "/api/downloads", {
      method: "POST",
      body: JSON.stringify({ url, adapter }),
    });
  },

  async listDownloads(): Promise<DownloadJob[]> {
    return localOrRemote("list_downloads", undefined, "/api/downloads");
  },

  async cancelDownload(id: string): Promise<void> {
    return localOrRemote("cancel_download", { id }, `/api/downloads/${id}/cancel`, {
      method: "POST",
    });
  },

  async listScenes(query?: string): Promise<Scene[]> {
    const q = query ? `?q=${encodeURIComponent(query)}` : "";
    return localOrRemote("list_scenes", { query }, `/api/scenes${q}`);
  },

  async listPerformers(query?: string): Promise<Performer[]> {
    const q = query ? `?q=${encodeURIComponent(query)}` : "";
    return localOrRemote("list_performers", { query }, `/api/performers${q}`);
  },

  async listTags(): Promise<Tag[]> {
    return localOrRemote("list_tags", undefined, "/api/tags");
  },

  async getSettings(): Promise<AppSettings> {
    if (getAppRuntime() === "desktop-tauri") {
      return localInvoke<AppSettings>("get_settings");
    }
    return useSettingsStore.getState().settings;
  },

  async getHostSettings(): Promise<AppSettings> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<AppSettings>("/api/settings");
    }
    return api.getSettings();
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    if (getAppRuntime() === "desktop-tauri") {
      await localInvoke("save_settings", { settings });
      return;
    }
    useSettingsStore.getState().updateSettings(settings);
  },

  async saveHostSettings(settings: AppSettings): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch<void>("/api/settings", {
        method: "PUT",
        body: JSON.stringify(settings),
      });
      return;
    }
    await api.saveSettings(settings);
  },

  async testRemoteConnection(host: string, token?: string): Promise<HealthResponse> {
    const trimmed = token?.trim();
    const res = await fetch(`${host.replace(/\/$/, "")}/api/health`, {
      headers: trimmed ? { Authorization: `Bearer ${trimmed}` } : {},
    });
    if (res.status === 401 && trimmed) {
      const retry = await fetch(`${host.replace(/\/$/, "")}/api/health`, { headers: {} });
      if (retry.ok) {
        useSettingsStore.getState().updateSettings({ remote_token: undefined });
        return retry.json() as Promise<HealthResponse>;
      }
    }
    if (!res.ok) throw new Error("Connection failed");
    const health = (await res.json()) as HealthResponse;
    if (health.auth_required === false) {
      useSettingsStore.getState().updateSettings({ remote_token: undefined });
    }
    return health;
  },

  async discoverLanHosts(timeoutMs = 6000): Promise<LanHost[]> {
    if (getAppRuntime() === "browser") {
      return [];
    }
    return localInvoke<LanHost[]>("discover_lan_hosts", { timeoutMs });
  },

  async startLanServer(port: number): Promise<{ token: string }> {
    if (getAppRuntime() !== "desktop-tauri") {
      throw new Error("Enable the LAN server on the desktop app (Settings → LAN).");
    }
    return localInvoke("start_lan_server", { port });
  },

  async stopLanServer(): Promise<void> {
    if (getAppRuntime() !== "desktop-tauri") return;
    await localInvoke("stop_lan_server");
  },

  async regenerateLanServer(port: number): Promise<{ token: string }> {
    if (getAppRuntime() !== "desktop-tauri") {
      throw new Error("Regenerate the LAN token on the desktop app (Settings → LAN).");
    }
    return localInvoke("regenerate_lan_server", { port });
  },

  async getScene(id: string): Promise<Scene> {
    return localOrRemote("get_scene", { id }, `/api/scenes/${id}`);
  },

  async updateScene(id: string, body: UpdateSceneRequest): Promise<Scene> {
    return localOrRemote("update_scene", { id, body }, `/api/scenes/${id}`, {
      method: "PATCH",
      body: JSON.stringify(body),
    });
  },

  async openSceneInExplorer(id: string): Promise<void> {
    return localInvoke("open_scene_in_explorer", { id });
  },

  async openSceneWithDefault(id: string): Promise<void> {
    return localInvoke("open_scene_with_default", { id });
  },

  async batchUpdateScenes(body: BatchUpdateScenesRequest): Promise<BatchUpdateScenesResult> {
    return localOrRemote("batch_update_scenes", { body }, "/api/scenes/batch", {
      method: "POST",
      body: JSON.stringify(body),
    });
  },

  async listPornhubCategories(
    orientation: BrowseOrientation = "straight",
  ): Promise<PornhubCategoryEntry[]> {
    return localOrRemote(
      "list_pornhub_categories",
      { orientation },
      `/api/sites/pornhub/categories?orientation=${orientation}`,
    );
  },

  async scanLibrary(): Promise<{ added: number; updated: number }> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<{ added: number; updated: number }>("/api/library/scan", {
        method: "POST",
      });
    }
    if (getAppRuntime() !== "desktop-tauri") {
      throw new Error("Library scan runs on the desktop host.");
    }
    return localInvoke("scan_library");
  },

  async findDuplicates(): Promise<DuplicateGroup[]> {
    return localOrRemote("find_duplicates", undefined, "/api/duplicates");
  },

  async mergeDuplicates(
    keepId: string,
    removeIds: string[],
    deleteFiles = false,
  ): Promise<MergeDuplicatesResult> {
    return localOrRemote(
      "merge_duplicates",
      { keepId, removeIds, deleteFiles },
      "/api/duplicates/merge",
      {
        method: "POST",
        body: JSON.stringify({
          keep_id: keepId,
          remove_ids: removeIds,
          delete_files: deleteFiles,
        }),
      },
    );
  },

  async listCookieSites(): Promise<CookieSiteInfo[]> {
    return localOrRemote("list_cookie_sites", undefined, "/api/cookies");
  },

  async saveSiteCookies(siteId: string, cookies: string): Promise<void> {
    return localOrRemote("save_site_cookies", { siteId, cookies }, `/api/cookies/${siteId}`, {
      method: "POST",
      body: JSON.stringify({ cookies }),
    });
  },

  async deleteSiteCookies(siteId: string): Promise<void> {
    return localOrRemote("delete_site_cookies", { siteId }, `/api/cookies/${siteId}`, {
      method: "DELETE",
    });
  },

  async resolveStandalone(url: string): Promise<MediaItem> {
    return localInvoke<MediaItem>("resolve_standalone", { url });
  },

  async subscribeDownloadProgress(onProgress: (job: DownloadJob) => void): Promise<() => void> {
    if (!isDesktopTauri() || shouldUseRemoteApi()) {
      return () => {};
    }
    const unlisten = await listen<DownloadJob>("download:progress", (event) => {
      onProgress(event.payload);
    });
    return unlisten;
  },

  async subscribeScanProgress(onProgress: (progress: ScanProgress) => void): Promise<() => void> {
    if (getAppRuntime() !== "desktop-tauri") {
      return () => {};
    }
    const unlisten = await listen<ScanProgress>("library:scan-progress", (event) => {
      onProgress(event.payload);
    });
    return unlisten;
  },
};
