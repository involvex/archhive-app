import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  BinaryVersions,
  BinaryLatestVersions,
  BinaryUpdateResult,
  BrowseKind,
  BrowseOrientation,
  BrowsePage,
  BulkImportResult,
  ClearThumbsResult,
  ThumbCacheStats,
  LibraryStorageStats,
  DownloadJob,
  DuplicateGroup,
  BrowseFilters,
  CookieLoginResult,
  CookieSiteInfo,
  FilesListResponse,
  DirBrowseResponse,
  SidecarProbe,
  FfmpegStatus,
  HealthResponse,
  LanHost,
  MediaItem,
  MergeDuplicatesResult,
  OrphanSidecar,
  LibraryStats,
  MarkWatchedResult,
  SavedSearch,
  CheckSavedSearchResult,
  WatchlistPollResult,
  WatchlistStatus,
  Performer,
  Scene,
  SceneFilter,
  Collection,
  CreateCollectionRequest,
  UpdateCollectionRequest,
  SiteCategory,
  SiteInfo,
  Tag,
  UpdateSceneRequest,
  BatchUpdateScenesRequest,
  BatchUpdateScenesResult,
  DurationProbeResult,
  PornhubCategoryEntry,
  SceneSort,
  ThumbGenResult,
  WatchProgress,
  DiagnosticsData,
  LogEntry,
  SettingsBackup,
  SettingsBackupImportOptions,
  SettingsBackupImportResult,
  StashBoxEndpoint,
  StashSceneMatch,
  OnlineCrawl,
  OnlineDetailsResult,
  OnlinePage,
  OnlineQuery,
  OnlineSearchInfo,
  OnlineTagCount,
  OnlineUpdateResult,
  StashPerformerMatch,
  ApplyStashMatchRequest,
  SubmitFingerprintsResult,
  UnenrichedScenesResult,
  RehashResult,
} from "../types";
import { getAppRuntime, shouldUseRemoteApi } from "../runtime";
import { vibrateTick } from "../haptics";
import { useSettingsStore } from "../stores/settings";
import { isDesktopTauri, isTauri } from "../tauri";

/**
 * True when a livestream failure means the room isn't watchable right now
 * (offline, private show, deleted) rather than a resolvable stream issue.
 * Callers show a friendly offline message instead of the embed iframe.
 */
export function isRoomOfflineError(msg: string): boolean {
  return /private show|room is offline|\boffline\b|not broadcasting|user not found|no longer live|stream has ended|room.+not found|could not find room/i.test(
    msg,
  );
}

/**
 * Guard scraped/resolved media URLs before they reach `<video src>` or
 * `<iframe src>`. Only absolute `http(s)` URLs are allowed; anything else
 * (`javascript:`, `data:`, `file:`, relative paths, empty) becomes `""` so
 * callers treat it as "no stream" and fall back to the embed/error path.
 * Non-web protocols (e.g. `blob:`, `data:media`) are rejected by design.
 */
export function safeMediaUrl(value: unknown): string {
  if (typeof value !== "string") return "";
  const trimmed = value.trim();
  if (!/^https?:\/\//i.test(trimmed)) return "";
  return trimmed;
}

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
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 15_000);
  const doFetch = (headers: HeadersInit) =>
    fetch(`${base}${path}`, {
      ...init,
      headers: { ...headers, ...init?.headers },
      signal: controller.signal,
    });

  let res: Response;
  try {
    res = await doFetch(remoteHeaders(true));
  } catch (e) {
    clearTimeout(timeoutId);
    const host = base.replace(/^https?:\/\//, "");
    const hint =
      e instanceof DOMException && e.name === "AbortError"
        ? `Request to ${host} timed out. Check PC IP and network.`
        : e instanceof TypeError
          ? `Cannot reach ${host}. Check PC IP, Windows Firewall (TCP 8787), same Wi‑Fi, and cleartext HTTP on Android.`
          : e instanceof Error
            ? e.message
            : "Network error";
    throw new Error(hint, e instanceof Error ? { cause: e } : undefined);
  }
  clearTimeout(timeoutId);
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
  if (getAppRuntime() === "browser") {
    throw new Error("This action requires the app runtime.");
  }
  return invoke(command, args) as Promise<T>;
}

function shouldUseDeviceCookieVault(): boolean {
  const runtime = getAppRuntime();
  const { settings } = useSettingsStore.getState();
  if (runtime === "mobile-tauri") {
    return settings.engine_mode === "local" || settings.engine_mode === "standalone";
  }
  return !shouldUseRemoteApi();
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

/** Mirrors `BrowseFilters::cache_key` in models.rs. */
export function browseFiltersKey(filters?: BrowseFilters): string {
  if (
    !filters ||
    !(
      filters.sort ||
      filters.period ||
      filters.duration ||
      filters.quality ||
      filters.include ||
      filters.exclude ||
      filters.tag ||
      filters.source
    )
  ) {
    return "";
  }
  const base = `|s=${filters.sort ?? ""}|p=${filters.period ?? ""}|d=${filters.duration ?? ""}|q=${filters.quality ?? ""}`;
  const withCats =
    filters.include || filters.exclude
      ? `${base}|i=${filters.include ?? ""}|x=${filters.exclude ?? ""}`
      : base;
  const withTag = filters.tag ? `${withCats}|t=${filters.tag}` : withCats;
  return filters.source ? `${withTag}|src=${filters.source}` : withTag;
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
    filters?: BrowseFilters,
  ): Promise<BrowsePage> {
    const orientParam = orientation ? `&orientation=${orientation}` : "";
    const filterParams = Object.entries(filters ?? {})
      .filter(([, v]) => Boolean(v))
      .map(([k, v]) => `&${k}=${encodeURIComponent(String(v))}`)
      .join("");
    const cacheArgs = {
      siteId,
      // Keep sorted/filtered pages apart in the offline cache.
      kind: `${kind}${browseFiltersKey(filters)}`,
      slug,
      page,
      orientation: orientation ?? null,
    };

    try {
      const result = await localOrRemote<BrowsePage>(
        "browse",
        { siteId, kind, slug, page, orientation, filters: filters ?? null },
        `/api/sites/${siteId}/browse?kind=${kind}&slug=${encodeURIComponent(slug)}&page=${page}${orientParam}${filterParams}`,
      );
      // Dual-write to device SQLite so Remote LAN listings survive going offline.
      if (isTauri() && !result.from_cache) {
        void invoke("put_browse_cache", {
          ...cacheArgs,
          payload: result,
          ttlSecs: 86_400,
        }).catch(() => {
          /* best-effort */
        });
      }
      return result;
    } catch (err) {
      if (isTauri()) {
        try {
          const cached = await invoke<BrowsePage | null>("get_browse_cache", {
            ...cacheArgs,
            allowStale: true,
          });
          if (cached?.items?.length) {
            return cached;
          }
        } catch {
          /* fall through */
        }
      }
      throw err;
    }
  },

  /** A site's category / tag index (empty when the site has none). */
  // ---- Online library: local copy of listing results ----

  async queryOnlineItems(query: OnlineQuery): Promise<OnlinePage> {
    return localOrRemote<OnlinePage>("query_online_items", { query }, "/api/online/query", {
      method: "POST",
      body: JSON.stringify(query),
    });
  },

  async onlineTagCounts(searchId?: string, limit = 400): Promise<OnlineTagCount[]> {
    const params = new URLSearchParams({ limit: String(limit) });
    if (searchId) params.set("search_id", searchId);
    return localOrRemote<OnlineTagCount[]>(
      "online_tag_counts",
      { searchId: searchId ?? null, limit },
      `/api/online/tags?${params}`,
    );
  },

  async onlineSearchInfo(): Promise<OnlineSearchInfo[]> {
    return localOrRemote<OnlineSearchInfo[]>(
      "online_search_info",
      undefined,
      "/api/online/searches",
    );
  },

  /** Fetch `pages` pages of a saved search from the site (slowly). */
  async updateOnlineSearch(id: string, pages: number): Promise<OnlineUpdateResult> {
    return localOrRemote<OnlineUpdateResult>(
      "update_online_search",
      { id, pages },
      `/api/online/searches/${encodeURIComponent(id)}/update`,
      { method: "POST", body: JSON.stringify({ pages }) },
    );
  },

  /** Tags / performers from each video's own page (max 25 per call). */
  async fetchOnlineDetails(keys: string[]): Promise<OnlineDetailsResult> {
    return localOrRemote<OnlineDetailsResult>(
      "fetch_online_details",
      { keys },
      "/api/online/details",
      {
        method: "POST",
        body: JSON.stringify({ keys }),
      },
    );
  },

  /** Remove entries from a saved search (or, without one, from the whole
   * library). They stay removed across updates / browsing. */
  /** Bookmark a browsed video (Online library → Bookmarks). Returns its key. */
  async bookmarkOnline(item: MediaItem): Promise<string> {
    return localOrRemote<string>("online_bookmark", { item }, "/api/online/bookmarks", {
      method: "POST",
      body: JSON.stringify({ item }),
    });
  },

  async unbookmarkOnline(url: string): Promise<void> {
    await localOrRemote<null>("online_unbookmark", { url }, "/api/online/bookmarks/remove", {
      method: "POST",
      body: JSON.stringify({ url }),
    });
  },

  async isOnlineBookmarked(url: string): Promise<boolean> {
    return localOrRemote<boolean>("online_is_bookmarked", { url }, "/api/online/bookmarks/check", {
      method: "POST",
      body: JSON.stringify({ url }),
    });
  },

  async deleteOnlineItems(searchId: string | undefined, keys: string[]): Promise<number> {
    if (shouldUseRemoteApi()) {
      const res = await remoteFetch<{ removed: number }>("/api/online/delete", {
        method: "POST",
        body: JSON.stringify({ search_id: searchId ?? null, keys }),
      });
      return res.removed;
    }
    return localInvoke<number>("delete_online_items", { searchId: searchId ?? null, keys });
  },

  /** Remove everything the query matches (all pages). */
  async deleteOnlineMatching(query: OnlineQuery): Promise<number> {
    if (shouldUseRemoteApi()) {
      const res = await remoteFetch<{ removed: number }>("/api/online/delete-matching", {
        method: "POST",
        body: JSON.stringify(query),
      });
      return res.removed;
    }
    return localInvoke<number>("delete_online_matching", { query });
  },

  /** Fetch every page of a saved search in the background (paced, pausing
   * on rate limits). `restart` starts over from page 1. */
  async startFullFetch(id: string, restart = false): Promise<OnlineCrawl> {
    return localOrRemote<OnlineCrawl>(
      "start_full_fetch",
      { id, restart },
      `/api/online/searches/${encodeURIComponent(id)}/fetch-all`,
      { method: "POST", body: JSON.stringify({ restart }) },
    );
  },

  async stopFullFetch(id: string): Promise<void> {
    await localOrRemote<unknown>(
      "stop_full_fetch",
      { id },
      `/api/online/searches/${encodeURIComponent(id)}/fetch-all/stop`,
      { method: "POST", body: "{}" },
    );
  },

  async listFullFetches(): Promise<OnlineCrawl[]> {
    return localOrRemote<OnlineCrawl[]>("list_full_fetches", undefined, "/api/online/crawls");
  },

  async clearOnlineHistory(): Promise<number> {
    if (shouldUseRemoteApi()) {
      const res = await remoteFetch<{ removed: number }>("/api/online/history", {
        method: "DELETE",
      });
      return res.removed;
    }
    return localInvoke<number>("clear_online_history");
  },

  /** Category / tag index from the app's cache; `refresh` asks the site again. */
  async listSiteCategories(
    siteId: string,
    group?: string,
    refresh = false,
  ): Promise<SiteCategory[]> {
    const params = new URLSearchParams();
    if (group) params.set("group", group);
    if (refresh) params.set("refresh", "1");
    const qs = params.toString() ? `?${params}` : "";
    return localOrRemote<SiteCategory[]>(
      "list_site_categories",
      { siteId, group: group ?? null, refresh },
      `/api/sites/${encodeURIComponent(siteId)}/categories${qs}`,
    );
  },

  /** Search phrases the site suggests (`remote_suggest` sites; cached). */
  async siteSuggest(
    siteId: string,
    text: string,
    orientation?: BrowseOrientation,
  ): Promise<string[]> {
    const params = new URLSearchParams({ q: text });
    if (orientation) params.set("orientation", orientation);
    return localOrRemote<string[]>(
      "site_suggest",
      { siteId, text, orientation: orientation ?? null },
      `/api/sites/${encodeURIComponent(siteId)}/suggest?${params}`,
    );
  },

  async queueDownload(url: string, adapter?: string, title?: string): Promise<DownloadJob> {
    const job = await localOrRemote<DownloadJob>(
      "queue_download",
      { url, adapter, title },
      "/api/downloads",
      {
        method: "POST",
        body: JSON.stringify({ url, adapter, title }),
      },
    );
    // Centralized haptic feedback for all queue-download paths (Q38/Q39).
    vibrateTick(10);
    return job;
  },

  async queueDownloads(urls: string[]): Promise<DownloadJob[]> {
    if (isDesktopTauri() && !shouldUseRemoteApi()) {
      const jobs = await invoke<DownloadJob[]>("queue_downloads", { urls });
      // Single vibrate per batch call (not per URL).
      vibrateTick(10);
      return jobs;
    }
    const jobs: DownloadJob[] = [];
    for (const url of urls) {
      try {
        jobs.push(await this.queueDownload(url));
      } catch (e) {
        console.error("queue failed", url, e);
      }
    }
    // Vibrate once for the whole batch (non-desktop path).
    if (jobs.length > 0) vibrateTick(10);
    return jobs;
  },

  async listDownloads(): Promise<DownloadJob[]> {
    return localOrRemote("list_downloads", undefined, "/api/downloads");
  },

  async cancelDownload(id: string): Promise<void> {
    return localOrRemote("cancel_download", { id }, `/api/downloads/${id}/cancel`, {
      method: "POST",
    });
  },

  async pauseDownload(id: string): Promise<void> {
    return localOrRemote("pause_download", { id }, `/api/downloads/${id}/pause`, {
      method: "POST",
    });
  },

  async resumeDownload(id: string): Promise<void> {
    return localOrRemote("resume_download", { id }, `/api/downloads/${id}/resume`, {
      method: "POST",
    });
  },

  async retryDownload(id: string): Promise<void> {
    return localOrRemote("retry_download", { id }, `/api/downloads/${id}/retry`, {
      method: "POST",
    });
  },

  async updateNetworkState(connectionType: string, metered: boolean): Promise<void> {
    return localOrRemote(
      "update_network_state",
      { connection_type: connectionType, metered },
      "/api/network/state",
      {
        method: "POST",
      },
    );
  },

  async getNetworkInfo(): Promise<{ connection_type: string; metered: boolean }> {
    return localOrRemote("get_network_info", undefined, "/api/network/info");
  },

  async deleteDownload(id: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<void>(`/api/downloads/${id}`, { method: "DELETE" });
    }
    return localInvoke("delete_download", { id });
  },

  async queueBulkImport(
    urls: string[],
    expandBrowse = true,
    importAll = false,
  ): Promise<BulkImportResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<BulkImportResult>("/api/downloads/bulk", {
        method: "POST",
        body: JSON.stringify({ urls, expand_browse: expandBrowse, import_all: importAll }),
      });
    }
    return localInvoke<BulkImportResult>("queue_bulk_import", {
      urls,
      expandBrowse,
      importAll,
    });
  },

  async listScenes(
    query?: string,
    sort: SceneSort = "newest",
    limit?: number,
    offset?: number,
  ): Promise<Scene[]> {
    const params = new URLSearchParams();
    if (query) params.set("q", query);
    if (sort && sort !== "newest") params.set("sort", sort);
    if (limit != null) params.set("limit", String(limit));
    if (offset != null) params.set("offset", String(offset));
    const qs = params.toString() ? `?${params}` : "";
    return localOrRemote("list_scenes", { query, sort, limit, offset }, `/api/scenes${qs}`);
  },

  async deleteScene(id: string, deleteFiles = false): Promise<void> {
    if (shouldUseRemoteApi()) {
      const qs = deleteFiles ? "?delete_files=true" : "?delete_files=false";
      return remoteFetch<void>(`/api/scenes/${id}${qs}`, { method: "DELETE" });
    }
    return localInvoke("delete_scene", { id, deleteFiles });
  },

  async generateMissingThumbs(): Promise<ThumbGenResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<ThumbGenResult>("/api/library/thumbs", {
        method: "POST",
      });
    }
    return localInvoke<ThumbGenResult>("generate_missing_thumbs");
  },

  /** (Re)compute MD5 + OSHASH for files missing them (post-fingerprint-fix repair). */
  async rehashSceneHashes(): Promise<RehashResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<RehashResult>("/api/library/rehash", {
        method: "POST",
      });
    }
    return localInvoke<RehashResult>("rehash_scene_hashes");
  },

  async probeSceneMetadata(sceneId: string): Promise<Scene> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<Scene>(`/api/scenes/${sceneId}/probe`, { method: "POST" });
    }
    return localInvoke<Scene>("probe_scene_metadata", { sceneId });
  },

  async probeLibraryDurations(concurrency = 2): Promise<DurationProbeResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<DurationProbeResult>("/api/library/probe-durations", {
        method: "POST",
      });
    }
    return localInvoke<DurationProbeResult>("probe_library_durations", { concurrency });
  },

  async ffmpegStatus(): Promise<FfmpegStatus> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<FfmpegStatus>("/api/library/ffmpeg-status");
    }
    return localInvoke<FfmpegStatus>("ffmpeg_status");
  },

  async binaryVersions(): Promise<BinaryVersions> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<BinaryVersions>("/api/system/versions");
    }
    return localInvoke<BinaryVersions>("binary_versions");
  },

  async clearAllThumbs(): Promise<ClearThumbsResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<ClearThumbsResult>("/api/library/thumbs", {
        method: "DELETE",
      });
    }
    return localInvoke<ClearThumbsResult>("clear_all_thumbs");
  },

  async recordWatchProgress(
    sceneId: string,
    positionSecs: number,
    durationSecs: number,
  ): Promise<WatchProgress> {
    const body = { position_secs: positionSecs, duration_secs: durationSecs };
    if (shouldUseRemoteApi()) {
      return remoteFetch<WatchProgress>(`/api/scenes/${sceneId}/watch`, {
        method: "POST",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<WatchProgress>("record_watch_progress", {
      sceneId,
      positionSecs,
      durationSecs,
    });
  },

  async getWatchProgress(sceneId: string): Promise<WatchProgress | null> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<WatchProgress | null>(`/api/scenes/${sceneId}/watch`);
    }
    return localInvoke<WatchProgress | null>("get_watch_progress", { sceneId });
  },

  async listWatchProgress(): Promise<WatchProgress[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<WatchProgress[]>("/api/watch-progress");
    }
    if (getAppRuntime() === "browser") return [];
    return localInvoke<WatchProgress[]>("list_watch_progress");
  },

  async markWatched(sceneIds: string[], watched: boolean): Promise<MarkWatchedResult> {
    const body = { scene_ids: sceneIds, watched };
    if (shouldUseRemoteApi()) {
      return remoteFetch<MarkWatchedResult>("/api/scenes/watch", {
        method: "POST",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<MarkWatchedResult>("mark_watched", { body });
  },

  async listWatchedSourceUrls(): Promise<string[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<string[]>("/api/library/watched-urls");
    }
    if (getAppRuntime() === "browser") return [];
    return localInvoke<string[]>("list_watched_source_urls");
  },

  async saveSearch(body: {
    name: string;
    site_id: string;
    kind: BrowseKind;
    slug: string;
    orientation?: BrowseOrientation;
    filters?: BrowseFilters;
  }): Promise<SavedSearch> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<SavedSearch>("/api/saved-searches", {
        method: "POST",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<SavedSearch>("save_search", { body });
  },

  async listSavedSearches(): Promise<SavedSearch[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<SavedSearch[]>("/api/saved-searches");
    }
    if (getAppRuntime() === "browser") return [];
    return localInvoke<SavedSearch[]>("list_saved_searches");
  },

  async deleteSavedSearch(id: string): Promise<boolean> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<boolean>(`/api/saved-searches/${id}`, { method: "DELETE" });
    }
    return localInvoke<boolean>("delete_saved_search", { id });
  },

  async checkSavedSearch(id: string): Promise<CheckSavedSearchResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<CheckSavedSearchResult>(`/api/saved-searches/${id}/check`, {
        method: "POST",
      });
    }
    return localInvoke<CheckSavedSearchResult>("check_saved_search", { id });
  },

  async setSavedSearchAutoQueue(id: string, autoQueue: boolean): Promise<boolean> {
    const body = { auto_queue: autoQueue };
    if (shouldUseRemoteApi()) {
      return remoteFetch<boolean>(`/api/saved-searches/${id}`, {
        method: "PATCH",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<boolean>("update_saved_search", { id, body });
  },

  async pollWatchlist(): Promise<WatchlistPollResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<WatchlistPollResult>("/api/watchlist/poll", { method: "POST" });
    }
    return localInvoke<WatchlistPollResult>("poll_watchlist");
  },

  async watchlistStatus(): Promise<WatchlistStatus> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<WatchlistStatus>("/api/watchlist/status");
    }
    if (getAppRuntime() === "browser") {
      return { auto_queue_count: 0, due_count: 0 };
    }
    return localInvoke<WatchlistStatus>("watchlist_status");
  },

  async dismissSavedSearchNews(id: string): Promise<boolean> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<boolean>(`/api/saved-searches/${id}/dismiss`, { method: "POST" });
    }
    return localInvoke<boolean>("dismiss_saved_search_news", { id });
  },

  async listScenesWithFilter(
    filter: SceneFilter,
    limit?: number,
    offset?: number,
  ): Promise<Scene[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<Scene[]>("/api/library/filter", {
        method: "POST",
        body: JSON.stringify({ ...filter, limit, offset }),
      });
    }
    return localInvoke<Scene[]>("list_scenes_with_filter", { filter, limit, offset });
  },

  async listCollections(): Promise<Collection[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<Collection[]>("/api/collections");
    }
    return localInvoke<Collection[]>("list_collections");
  },

  async createCollection(req: CreateCollectionRequest): Promise<string> {
    if (shouldUseRemoteApi()) {
      const res = await remoteFetch<{ id: string }>("/api/collections", {
        method: "POST",
        body: JSON.stringify(req),
      });
      return res.id;
    }
    // Local IPC returns the bare id string (see commands::create_collection).
    return localInvoke<string>("create_collection", { req });
  },

  async deleteCollection(id: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch(`/api/collections/${id}`, { method: "DELETE" });
      return;
    }
    return localInvoke("delete_collection", { id });
  },

  async updateCollection(id: string, req: UpdateCollectionRequest): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch(`/api/collections/${id}`, {
        method: "PATCH",
        body: JSON.stringify(req),
      });
      return;
    }
    return localInvoke("update_collection", { id, req });
  },

  async listCollectionScenes(collectionId: string): Promise<Scene[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<Scene[]>(`/api/collections/${collectionId}/scenes`);
    }
    return localInvoke<Scene[]>("list_collection_scenes", { collection_id: collectionId });
  },

  async addSceneToCollection(sceneId: string, collectionId: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch(`/api/collections/${collectionId}/scenes/${sceneId}`, {
        method: "POST",
      });
      return;
    }
    return localInvoke("add_scene_to_collection", {
      scene_id: sceneId,
      collection_id: collectionId,
    });
  },

  async removeSceneFromCollection(sceneId: string, collectionId: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch(`/api/collections/${collectionId}/scenes/${sceneId}`, {
        method: "DELETE",
      });
      return;
    }
    return localInvoke("remove_scene_from_collection", {
      scene_id: sceneId,
      collection_id: collectionId,
    });
  },

  async sceneCollectionIds(sceneId: string): Promise<string[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<string[]>(`/api/scenes/${sceneId}/collections`);
    }
    return localInvoke<string[]>("scene_collection_ids", { scene_id: sceneId });
  },

  async exportCollection(
    collectionId: string,
    format: "m3u" = "m3u",
  ): Promise<{ content: string; filename: string }> {
    if (shouldUseRemoteApi()) {
      const response = await fetch(`/api/collections/${collectionId}/export?format=${format}`, {
        method: "GET",
      });
      if (!response.ok) {
        throw new Error("Export failed");
      }
      const blob = await response.blob();
      const content = await blob.text();
      const filename =
        response.headers.get("content-disposition")?.match(/filename="(.+)"/)?.[1] ||
        `${collectionId}.m3u`;
      return { content, filename };
    }
    return localInvoke<{ content: string; filename: string }>("export_collection", {
      collection_id: collectionId,
      req: { format },
    });
  },

  async listOrphanSidecars(): Promise<OrphanSidecar[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<OrphanSidecar[]>("/api/library/orphans");
    }
    return localInvoke<OrphanSidecar[]>("list_orphan_sidecars");
  },

  /** Q43: thumbnail sidecar cache totals for the Settings storage row. */
  async thumbCacheStats(): Promise<ThumbCacheStats> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<ThumbCacheStats>("/api/library/thumb-cache");
    }
    return localInvoke<ThumbCacheStats>("thumb_cache_stats");
  },

  /** Single-pass orphan + thumb-cache scan for Settings → Library. */
  async libraryStorageStats(): Promise<LibraryStorageStats> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<LibraryStorageStats>("/api/library/storage-stats");
    }
    return localInvoke<LibraryStorageStats>("library_storage_stats");
  },

  async deleteOrphanSidecar(path: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch<void>(`/api/library/orphans?path=${encodeURIComponent(path)}`, {
        method: "DELETE",
      });
      return;
    }
    return localInvoke<void>("delete_orphan_sidecar", { path });
  },

  async clearSceneThumb(sceneId: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch<void>(`/api/scenes/${sceneId}/thumb`, { method: "DELETE" });
      return;
    }
    return localInvoke<void>("clear_scene_thumb", { sceneId });
  },

  async resolveMediaDetails(url: string): Promise<MediaItem> {
    return localOrRemote("resolve_media_details", { url }, "/api/media/resolve", {
      method: "POST",
      body: JSON.stringify({ url }),
    });
  },

  async resolveStreamUrl(url: string): Promise<string> {
    const raw = shouldUseRemoteApi()
      ? (
          await remoteFetch<{ stream_url: string }>("/api/media/stream-url", {
            method: "POST",
            body: JSON.stringify({ url }),
          })
        ).stream_url
      : await localInvoke<string>("resolve_stream_url", { url });
    return safeMediaUrl(raw);
  },

  async resolveLivestream(url: string): Promise<{
    stream_url: string;
    audio_url: string;
    embed_url: string;
    stream_error: string;
  }> {
    const raw = shouldUseRemoteApi()
      ? await remoteFetch<{
          stream_url: string;
          audio_url?: string | null;
          embed_url: string;
          stream_error?: string;
        }>("/api/media/livestream", {
          method: "POST",
          body: JSON.stringify({ url }),
        })
      : await localInvoke<{
          stream_url: string;
          audio_url?: string | null;
          embed_url: string;
          stream_error?: string;
        }>("resolve_livestream", {
          url,
        });
    // Security: embed/stream URLs land in <iframe src> / <video src>.
    return {
      stream_url: safeMediaUrl(raw.stream_url),
      audio_url: safeMediaUrl(raw.audio_url ?? ""),
      embed_url: safeMediaUrl(raw.embed_url),
      stream_error: typeof raw.stream_error === "string" ? raw.stream_error : "",
    };
  },

  async ensurePerformer(name: string): Promise<Performer> {
    return localOrRemote("ensure_performer", { name }, "/api/performers/ensure", {
      method: "POST",
      body: JSON.stringify({ name }),
    });
  },

  async listPerformers(
    query?: string,
    opts?: { cup_size?: string; hair_color?: string },
  ): Promise<Performer[]> {
    const params = new URLSearchParams();
    if (query) params.set("q", query);
    if (opts?.cup_size) params.set("cup_size", opts.cup_size);
    if (opts?.hair_color) params.set("hair_color", opts.hair_color);
    const qs = params.toString();
    const suffix = qs ? `?${qs}` : "";
    return localOrRemote(
      "list_performers",
      { query, cup_size: opts?.cup_size, hair_color: opts?.hair_color },
      `/api/performers${suffix}`,
    );
  },

  async updatePerformerAttributes(
    id: string,
    attrs: { cup_size?: string | null; hair_color?: string | null },
  ): Promise<void> {
    // `null`/empty clears the attribute (stored as NULL). Undefined leaves it out.
    const body = {
      cup_size: attrs.cup_size ?? undefined,
      hair_color: attrs.hair_color ?? undefined,
    };
    if (shouldUseRemoteApi()) {
      await remoteFetch(`/api/performers/${id}/attributes`, {
        method: "PUT",
        body: JSON.stringify(body),
      });
      return;
    }
    await localInvoke("update_performer_attributes", { id, ...body });
  },

  async listTags(): Promise<Tag[]> {
    return localOrRemote("list_tags", undefined, "/api/tags");
  },

  /** Always reads on-device SQLite (ignores Remote LAN routing). */
  async getDeviceSettings(): Promise<AppSettings> {
    if (getAppRuntime() === "browser") {
      return useSettingsStore.getState().settings;
    }
    return localInvoke<AppSettings>("get_settings");
  },

  /** Always writes on-device SQLite (ignores Remote LAN routing). */
  async saveDeviceSettings(settings: AppSettings): Promise<void> {
    if (getAppRuntime() === "browser") {
      useSettingsStore.getState().updateSettings(settings);
      return;
    }
    await localInvoke("save_settings", { settings });
  },

  async getSettings(): Promise<AppSettings> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<AppSettings>("/api/settings");
    }
    if (getAppRuntime() === "browser") {
      return useSettingsStore.getState().settings;
    }
    return localInvoke<AppSettings>("get_settings");
  },

  async getHostSettings(): Promise<AppSettings> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<AppSettings>("/api/settings");
    }
    return api.getSettings();
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch<void>("/api/settings", {
        method: "PUT",
        body: JSON.stringify(settings),
      });
      return;
    }
    if (getAppRuntime() === "browser") {
      useSettingsStore.getState().updateSettings(settings);
      return;
    }
    await localInvoke("save_settings", { settings });
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

  async listFiles(path = ""): Promise<FilesListResponse> {
    const q = path ? `?path=${encodeURIComponent(path)}` : "";
    if (shouldUseRemoteApi()) {
      return remoteFetch<FilesListResponse>(`/api/files${q}`);
    }
    if (getAppRuntime() === "browser") {
      throw new Error("File browsing requires the app runtime.");
    }
    return localInvoke<FilesListResponse>("list_files", { path });
  },

  async scanLibrary(): Promise<{ added: number; updated: number }> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<{ added: number; updated: number }>("/api/library/scan", {
        method: "POST",
      });
    }
    if (getAppRuntime() === "browser") {
      throw new Error("Library scan requires the app runtime.");
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
    // Local/Standalone on mobile always use the on-device vault.
    if (shouldUseDeviceCookieVault()) {
      return localInvoke<CookieSiteInfo[]>("list_cookie_sites");
    }
    return remoteFetch<CookieSiteInfo[]>("/api/cookies");
  },

  async saveSiteCookies(siteId: string, cookies: string): Promise<void> {
    if (shouldUseDeviceCookieVault()) {
      await localInvoke("save_site_cookies", { siteId, cookies });
      return;
    }
    await remoteFetch<void>(`/api/cookies/${siteId}`, {
      method: "POST",
      body: JSON.stringify({ cookies }),
    });
  },

  /**
   * Open the in-app login browser for `siteId` and store the captured cookies
   * (on this device, or on the LAN host in Remote LAN mode). Resolves once the
   * user saves; rejects with "Login cancelled." when they back out.
   */
  async loginForCookies(siteId: string, url?: string): Promise<CookieLoginResult> {
    // `url`: required for per-domain entries (`host-<domain>`), which have
    // no adapter to supply the site's address.
    const result = await localInvoke<CookieLoginResult>("cookie_login", {
      siteId,
      url: url ?? null,
    });
    await api.saveSiteCookies(siteId, result.netscape);
    return result;
  },

  /** Desktop: save (`true`) or cancel (`false`) the open login window. */
  async finishCookieLogin(save: boolean): Promise<void> {
    await localInvoke("cookie_login_finish", { save });
  },

  async deleteSiteCookies(siteId: string): Promise<void> {
    if (shouldUseDeviceCookieVault()) {
      await localInvoke("delete_site_cookies", { siteId });
      return;
    }
    await remoteFetch<void>(`/api/cookies/${siteId}`, { method: "DELETE" });
  },

  async resolveStandalone(url: string): Promise<MediaItem> {
    return localInvoke<MediaItem>("resolve_standalone", { url });
  },

  async subscribeDownloadProgress(onProgress: (job: DownloadJob) => void): Promise<() => void> {
    if (shouldUseRemoteApi()) {
      return () => {};
    }
    if (getAppRuntime() === "browser") {
      return () => {};
    }
    const unlisten = await listen<DownloadJob>("download:progress", (event) => {
      onProgress(event.payload);
    });
    return unlisten;
  },

  async subscribeScanProgress(onProgress: (progress: ScanProgress) => void): Promise<() => void> {
    if (shouldUseRemoteApi()) {
      return () => {};
    }
    if (getAppRuntime() === "browser") {
      return () => {};
    }
    const unlisten = await listen<ScanProgress>("library:scan-progress", (event) => {
      onProgress(event.payload);
    });
    return unlisten;
  },

  async getLibraryStats(): Promise<LibraryStats> {
    return localInvoke<LibraryStats>("get_library_stats");
  },

  async getDiagnostics(): Promise<DiagnosticsData> {
    // Mobile always reports the device (engine mode + on-device cookie vault).
    if (getAppRuntime() === "mobile-tauri") {
      return localInvoke<DiagnosticsData>("get_diagnostics");
    }
    if (shouldUseRemoteApi()) {
      return remoteFetch<DiagnosticsData>("/api/diagnostics");
    }
    if (getAppRuntime() === "browser") {
      throw new Error("Diagnostics require the app runtime.");
    }
    return localInvoke<DiagnosticsData>("get_diagnostics");
  },

  /** Export settings + cookies from the on-device vault (cross-device portable). */
  async exportSettingsBackup(): Promise<SettingsBackup> {
    if (getAppRuntime() === "browser") {
      throw new Error("Backup export requires the app runtime.");
    }
    // Always export from this device (not the remote host).
    return localInvoke<SettingsBackup>("export_settings_backup");
  },

  async importSettingsBackup(
    backup: SettingsBackup,
    options?: SettingsBackupImportOptions,
  ): Promise<SettingsBackupImportResult> {
    if (getAppRuntime() === "browser") {
      throw new Error("Backup import requires the app runtime.");
    }
    return localInvoke<SettingsBackupImportResult>("import_settings_backup", {
      backup,
      options: options ?? { include_remote_credentials: false },
    });
  },

  async getRecentLogs(limit?: number): Promise<LogEntry[]> {
    if (shouldUseRemoteApi()) {
      const qs = limit ? `?limit=${limit}` : "";
      return remoteFetch<LogEntry[]>(`/api/logs${qs}`);
    }
    return localInvoke<LogEntry[]>("get_recent_logs", { limit });
  },

  async clearLogs(): Promise<void> {
    if (shouldUseRemoteApi()) {
      return remoteFetch("/api/logs/clear", { method: "POST" });
    }
    return localInvoke("clear_logs");
  },

  async exportPerformers(): Promise<Performer[]> {
    return localOrRemote("export_performers", undefined, "/api/performers");
  },

  async setPerformerImage(id: string, image: string | null): Promise<void> {
    return localOrRemote("set_performer_image", { id, image }, `/api/performers/${id}/image`, {
      method: "PUT",
      body: JSON.stringify({ image }),
    });
  },

  async installYtDlp(): Promise<{ installed: boolean; path?: string }> {
    if (shouldUseRemoteApi()) {
      throw new Error("Install binaries on the local device, not via Remote LAN.");
    }
    return localInvoke("install_yt_dlp");
  },

  async installGalleryDl(): Promise<{ installed: boolean; path?: string }> {
    if (shouldUseRemoteApi()) {
      throw new Error("Install binaries on the local device, not via Remote LAN.");
    }
    return localInvoke("install_gallery_dl");
  },

  async getInstalledBinaries(): Promise<BinaryVersions> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<BinaryVersions>("/api/system/installed-binaries");
    }
    return localInvoke<BinaryVersions>("get_installed_binaries");
  },

  async uninstallBinary(name: string): Promise<boolean> {
    if (shouldUseRemoteApi()) {
      throw new Error("Uninstall binaries on the local device, not via Remote LAN.");
    }
    return localInvoke<boolean>("uninstall_binary", { name });
  },

  async updateYtDlp(): Promise<string> {
    if (shouldUseRemoteApi()) {
      throw new Error("Update the yt-dlp engine on the local device, not via Remote LAN.");
    }
    return localInvoke<string>("update_yt_dlp");
  },

  /**
   * Android "All files access" (deleting library files in shared storage
   * that this install didn't create). Always true off the phone app.
   */
  async storageAccess(): Promise<boolean> {
    if (getAppRuntime() !== "mobile-tauri") return true;
    try {
      return await localInvoke<boolean>("storage_access_status");
    } catch {
      return true;
    }
  },

  /** Android: open the system "All files access" screen for ArcHive. */
  async openStorageAccessSettings(): Promise<void> {
    await localInvoke("open_storage_access_settings");
  },

  async defaultLibraryDir(): Promise<string> {
    if (shouldUseRemoteApi()) {
      throw new Error("Pick the library folder on the host device, not via Remote LAN.");
    }
    if (getAppRuntime() === "browser") {
      throw new Error("Folder picking requires the app runtime.");
    }
    return localInvoke<string>("default_library_dir");
  },

  async browseDirs(path?: string): Promise<DirBrowseResponse> {
    if (shouldUseRemoteApi()) {
      throw new Error("Browse folders on the local device, not via Remote LAN.");
    }
    if (getAppRuntime() === "browser") {
      throw new Error("Folder browsing requires the app runtime.");
    }
    return localInvoke("browse_dirs", { path: path ?? null });
  },

  async probeSidecar(name: string): Promise<SidecarProbe> {
    if (shouldUseRemoteApi()) {
      throw new Error("Probe sidecars on the local device, not via Remote LAN.");
    }
    if (getAppRuntime() === "browser") {
      throw new Error("Sidecar probing requires the app runtime.");
    }
    return localInvoke("probe_sidecar", { name });
  },

  async checkBinaryUpdates(): Promise<BinaryLatestVersions> {
    if (getAppRuntime() === "browser") {
      throw new Error("Binary update checks require the app runtime.");
    }
    return localInvoke("check_binary_updates", {});
  },

  async updateBinary(name: string): Promise<BinaryUpdateResult> {
    if (getAppRuntime() === "browser") {
      throw new Error("Binary updates require the app runtime.");
    }
    return localInvoke("update_binary", { name });
  },

  async rollbackBinary(name: string): Promise<BinaryUpdateResult> {
    if (getAppRuntime() === "browser") {
      throw new Error("Binary rollback requires the app runtime.");
    }
    return localInvoke("rollback_binary", { name });
  },

  async loadDemoScene(): Promise<string> {
    if (shouldUseRemoteApi()) {
      throw new Error("Load the demo scene on the local device, not via Remote LAN.");
    }
    return localInvoke<string>("load_demo_scene");
  },

  // ---- StashDB / stash-box enrichment (vault-backed API keys) ----

  /** Endpoints only expose has_key — keys never leave the host vault. */
  async listStashboxEndpoints(): Promise<StashBoxEndpoint[]> {
    return localOrRemote("list_stashbox_endpoints", undefined, "/api/stashbox/endpoints");
  },

  async saveStashboxEndpoint(args: {
    id?: string;
    name: string;
    endpoint: string;
    apiKey?: string;
  }): Promise<StashBoxEndpoint> {
    const { id, name, endpoint, apiKey } = args;
    if (shouldUseRemoteApi()) {
      return remoteFetch<StashBoxEndpoint>("/api/stashbox/endpoints", {
        method: "POST",
        body: JSON.stringify({ id, name, endpoint, api_key: apiKey }),
      });
    }
    return localInvoke<StashBoxEndpoint>("save_stashbox_endpoint", {
      id: id ?? null,
      name,
      endpoint,
      apiKey: apiKey ?? null,
    });
  },

  async deleteStashboxEndpoint(id: string): Promise<void> {
    if (shouldUseRemoteApi()) {
      await remoteFetch<void>(`/api/stashbox/endpoints/${id}`, { method: "DELETE" });
      return;
    }
    await localInvoke("delete_stashbox_endpoint", { id });
  },

  async testStashboxEndpoint(id: string): Promise<string> {
    if (shouldUseRemoteApi()) {
      const res = await remoteFetch<{ account: string }>(`/api/stashbox/endpoints/${id}/test`, {
        method: "POST",
      });
      return res.account;
    }
    return localInvoke<string>("test_stashbox_endpoint", { id });
  },

  /** Fingerprint-first, then a scored metadata match (title, performers,
   * studio, length) unless `fingerprintOnly`. `minScore` drops weaker
   * metadata candidates. Works local + Remote LAN. */
  async queryStashdbForScene(
    sceneId: string,
    endpointId?: string,
    fingerprintOnly = false,
    minScore?: number,
  ): Promise<StashSceneMatch[]> {
    const body = {
      scene_id: sceneId,
      endpoint_id: endpointId ?? null,
      fingerprint_only: fingerprintOnly,
      min_score: minScore ?? null,
    };
    if (shouldUseRemoteApi()) {
      return remoteFetch<StashSceneMatch[]>("/api/stashbox/query", {
        method: "POST",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<StashSceneMatch[]>("query_stashdb_for_scene", {
      sceneId,
      endpointId: endpointId ?? null,
      fingerprintOnly,
      minScore: minScore ?? null,
    });
  },

  /** Scenes with fingerprints but no StashDB link yet (batch Identify). */
  async listUnenrichedScenes(limit = 25): Promise<UnenrichedScenesResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<UnenrichedScenesResult>(`/api/stashbox/unenriched?limit=${limit}`);
    }
    return localInvoke<UnenrichedScenesResult>("list_unenriched_scenes", { limit });
  },

  /** Text search; with `sceneId` results are ranked against that scene. */
  async searchStashdbScenes(
    title: string,
    endpointId?: string,
    sceneId?: string,
  ): Promise<StashSceneMatch[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<StashSceneMatch[]>("/api/stashbox/search-scenes", {
        method: "POST",
        body: JSON.stringify({
          title,
          endpoint_id: endpointId ?? null,
          scene_id: sceneId ?? null,
        }),
      });
    }
    return localInvoke<StashSceneMatch[]>("search_stashdb_scenes", {
      title,
      endpointId: endpointId ?? null,
      sceneId: sceneId ?? null,
    });
  },

  async searchStashdbPerformers(name: string, endpointId?: string): Promise<StashPerformerMatch[]> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<StashPerformerMatch[]>("/api/stashbox/search-performers", {
        method: "POST",
        body: JSON.stringify({ name, endpoint_id: endpointId ?? null }),
      });
    }
    return localInvoke<StashPerformerMatch[]>("search_stashdb_performers", {
      name,
      endpointId: endpointId ?? null,
    });
  },

  async applyStashdbMatch(body: ApplyStashMatchRequest): Promise<Scene> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<Scene>("/api/stashbox/apply", {
        method: "POST",
        body: JSON.stringify(body),
      });
    }
    return localInvoke<Scene>("apply_stashdb_match", { body });
  },

  async submitStashdbFingerprints(
    sceneId: string,
    endpointId?: string,
  ): Promise<SubmitFingerprintsResult> {
    if (shouldUseRemoteApi()) {
      return remoteFetch<SubmitFingerprintsResult>("/api/stashbox/submit", {
        method: "POST",
        body: JSON.stringify({ scene_id: sceneId, endpoint_id: endpointId ?? null }),
      });
    }
    return localInvoke<SubmitFingerprintsResult>("submit_stashdb_fingerprints", {
      sceneId,
      endpointId: endpointId ?? null,
    });
  },

  /** Link a performer to a StashDB id (merges aliases, sets portrait if empty). */
  async linkPerformerStash(args: {
    performerId: string;
    stashId: string;
    image?: string;
    aliases?: string[];
  }): Promise<Performer> {
    const { performerId, stashId, image, aliases } = args;
    if (shouldUseRemoteApi()) {
      return remoteFetch<Performer>("/api/stashbox/link-performer", {
        method: "POST",
        body: JSON.stringify({
          performer_id: performerId,
          stash_id: stashId,
          image: image ?? null,
          aliases: aliases ?? [],
        }),
      });
    }
    return localInvoke<Performer>("link_performer_stash", {
      performerId,
      stashId,
      image: image ?? null,
      aliases: aliases ?? [],
    });
  },
};
