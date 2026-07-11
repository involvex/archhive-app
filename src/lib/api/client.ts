import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  BrowseKind,
  BrowsePage,
  DownloadJob,
  DuplicateGroup,
  CookieSiteInfo,
  HealthResponse,
  MediaItem,
  Performer,
  Scene,
  SiteInfo,
  Tag,
} from "../types";
import { useSettingsStore } from "../stores/settings";

function getRemoteBase(): string | null {
  const { settings } = useSettingsStore.getState();
  if (settings.engine_mode !== "remote_lan" || !settings.remote_host) return null;
  return settings.remote_host.replace(/\/$/, "");
}

function remoteHeaders(): HeadersInit {
  const { settings } = useSettingsStore.getState();
  const headers: HeadersInit = { "Content-Type": "application/json" };
  if (settings.remote_token) {
    headers["Authorization"] = `Bearer ${settings.remote_token}`;
  }
  return headers;
}

async function remoteFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const base = getRemoteBase();
  if (!base) throw new Error("Remote host not configured");
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: { ...remoteHeaders(), ...init?.headers },
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  return res.json() as Promise<T>;
}

async function localOrRemote<T>(
  localFn: () => Promise<T>,
  remotePath: string,
  init?: RequestInit,
): Promise<T> {
  const { settings } = useSettingsStore.getState();
  if (settings.engine_mode === "remote_lan") {
    return remoteFetch<T>(remotePath, init);
  }
  return localFn();
}

export const api = {
  async health(): Promise<HealthResponse> {
    return localOrRemote(() => invoke<HealthResponse>("health"), "/api/health");
  },

  async listSites(): Promise<SiteInfo[]> {
    return localOrRemote(() => invoke<SiteInfo[]>("list_sites"), "/api/sites");
  },

  async browse(siteId: string, kind: BrowseKind, slug: string, page = 1): Promise<BrowsePage> {
    return localOrRemote(
      () => invoke<BrowsePage>("browse", { siteId, kind, slug, page }),
      `/api/sites/${siteId}/browse?kind=${kind}&slug=${encodeURIComponent(slug)}&page=${page}`,
    );
  },

  async queueDownload(url: string, adapter?: string): Promise<DownloadJob> {
    return localOrRemote(
      () => invoke<DownloadJob>("queue_download", { url, adapter }),
      "/api/downloads",
      { method: "POST", body: JSON.stringify({ url, adapter }) },
    );
  },

  async listDownloads(): Promise<DownloadJob[]> {
    return localOrRemote(() => invoke<DownloadJob[]>("list_downloads"), "/api/downloads");
  },

  async cancelDownload(id: string): Promise<void> {
    return localOrRemote(() => invoke("cancel_download", { id }), `/api/downloads/${id}/cancel`, {
      method: "POST",
    });
  },

  async listScenes(query?: string): Promise<Scene[]> {
    const q = query ? `?q=${encodeURIComponent(query)}` : "";
    return localOrRemote(() => invoke<Scene[]>("list_scenes", { query }), `/api/scenes${q}`);
  },

  async listPerformers(query?: string): Promise<Performer[]> {
    const q = query ? `?q=${encodeURIComponent(query)}` : "";
    return localOrRemote(
      () => invoke<Performer[]>("list_performers", { query }),
      `/api/performers${q}`,
    );
  },

  async listTags(): Promise<Tag[]> {
    return localOrRemote(() => invoke<Tag[]>("list_tags"), "/api/tags");
  },

  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>("get_settings");
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    await invoke("save_settings", { settings });
  },

  async testRemoteConnection(host: string, token?: string): Promise<HealthResponse> {
    const res = await fetch(`${host.replace(/\/$/, "")}/api/health`, {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    });
    if (!res.ok) throw new Error("Connection failed");
    return res.json() as Promise<HealthResponse>;
  },

  async startLanServer(port: number): Promise<{ token: string }> {
    return invoke("start_lan_server", { port });
  },

  async stopLanServer(): Promise<void> {
    await invoke("stop_lan_server");
  },

  async scanLibrary(): Promise<{ added: number; updated: number }> {
    return invoke("scan_library");
  },

  async findDuplicates(): Promise<DuplicateGroup[]> {
    return localOrRemote(() => invoke<DuplicateGroup[]>("find_duplicates"), "/api/duplicates");
  },

  async listCookieSites(): Promise<CookieSiteInfo[]> {
    return localOrRemote(() => invoke<CookieSiteInfo[]>("list_cookie_sites"), "/api/cookies");
  },

  async saveSiteCookies(siteId: string, cookies: string): Promise<void> {
    return localOrRemote(
      () => invoke("save_site_cookies", { siteId, cookies }),
      `/api/cookies/${siteId}`,
      { method: "POST", body: JSON.stringify({ cookies }) },
    );
  },

  async deleteSiteCookies(siteId: string): Promise<void> {
    return localOrRemote(
      () => invoke("delete_site_cookies", { siteId }),
      `/api/cookies/${siteId}`,
      { method: "DELETE" },
    );
  },

  async resolveStandalone(url: string): Promise<MediaItem> {
    return invoke<MediaItem>("resolve_standalone", { url });
  },

  subscribeDownloadProgress(onProgress: (job: DownloadJob) => void): Promise<() => void> {
    return listen<DownloadJob>("download:progress", (event) => {
      onProgress(event.payload);
    }).then((unlisten) => unlisten);
  },
};
