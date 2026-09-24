import { describe, it, expect, vi, beforeEach } from "vitest";
import { api } from "@/lib/api/client";
import { shouldUseRemoteApi } from "@/lib/runtime";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@/lib/runtime", () => ({
  shouldUseRemoteApi: vi.fn(),
  getAppRuntime: vi.fn(),
}));

vi.mock("@/lib/stores/settings", () => ({
  useSettingsStore: {
    getState: vi.fn(),
  },
}));

import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/lib/stores/settings";

const mockInvoke = vi.mocked(invoke);
const mockShouldUseRemoteApi = vi.mocked(shouldUseRemoteApi);
const mockUseSettingsStore = vi.mocked(useSettingsStore.getState);

const baseSettings = {
  engine_mode: "local" as const,
  library_path: "",
  naming_template: "",
  lan_enabled: false,
  lan_port: 8787,
  remote_host: undefined,
  remote_token: undefined,
  download_quality: "1080" as const,
  prefer_mp4: true,
  auto_advance_next: false,
  theme_schedule_from: "19:00",
  theme_schedule_to: "07:00",
};

const baseStoreState = {
  settings: baseSettings,
  hydrated: true,
  setHydrated: vi.fn(),
  setEngineMode: vi.fn(),
  setRemoteHost: vi.fn(),
  setRemoteToken: vi.fn(),
  setLanEnabled: vi.fn(),
  setLanPort: vi.fn(),
  setLibraryPath: vi.fn(),
  updateSettings: vi.fn(),
};

const createMockFetch = (jsonData: unknown) => {
  const headers = new Headers();
  headers.set("content-type", "application/json");
  return vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    headers,
    json: () => Promise.resolve(jsonData),
  });
};

describe("api client - diagnostics", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockShouldUseRemoteApi.mockReturnValue(false);
    mockUseSettingsStore.mockReturnValue(baseStoreState);
    mockInvoke.mockResolvedValue({} as Record<string, unknown>);
  });

  describe("getDiagnostics", () => {
    it("calls local invoke on desktop mode", async () => {
      mockShouldUseRemoteApi.mockReturnValue(false);
      const mockData = { app_version: "0.4.0", recent_logs: [] };
      mockInvoke.mockResolvedValue(mockData);

      const result = await api.getDiagnostics();

      expect(mockInvoke).toHaveBeenCalledWith("get_diagnostics", undefined);
      expect(result).toEqual(mockData);
    });

    it("calls remote fetch on LAN mode", async () => {
      mockShouldUseRemoteApi.mockReturnValue(true);
      mockUseSettingsStore.mockReturnValue({
        ...baseStoreState,
        settings: {
          ...baseSettings,
          engine_mode: "remote_lan",
          lan_enabled: true,
          remote_host: "http://localhost:8787",
          remote_token: "test-token",
        },
      });

      globalThis.fetch = createMockFetch({ app_version: "0.4.0", recent_logs: [] });

      const result = await api.getDiagnostics();

      expect(fetch).toHaveBeenCalledWith("http://localhost:8787/api/diagnostics", {
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer test-token",
        },
        signal: expect.any(AbortSignal),
      });
      expect(result.app_version).toBe("0.4.0");
    });

    it("throws when remote host not configured", async () => {
      mockShouldUseRemoteApi.mockReturnValue(true);
      mockUseSettingsStore.mockReturnValue({
        ...baseStoreState,
        settings: {
          ...baseSettings,
          engine_mode: "remote_lan",
          lan_enabled: true,
          remote_host: undefined,
          remote_token: undefined,
        },
      });

      await expect(api.getDiagnostics()).rejects.toThrow("Remote host not configured");
    });
  });

  describe("getRecentLogs", () => {
    it("calls local invoke with limit", async () => {
      mockShouldUseRemoteApi.mockReturnValue(false);
      const mockLogs = [{ timestamp: Date.now(), level: "INFO", message: "test" }];
      mockInvoke.mockResolvedValue(mockLogs);

      const result = await api.getRecentLogs(100);

      expect(mockInvoke).toHaveBeenCalledWith("get_recent_logs", { limit: 100 });
      expect(result).toEqual(mockLogs);
    });

    it("calls remote fetch with query string", async () => {
      mockShouldUseRemoteApi.mockReturnValue(true);
      mockUseSettingsStore.mockReturnValue({
        ...baseStoreState,
        settings: {
          ...baseSettings,
          engine_mode: "remote_lan",
          lan_enabled: true,
          remote_host: "http://localhost:8787",
          remote_token: "test-token",
        },
      });

      globalThis.fetch = createMockFetch([
        { timestamp: Date.now(), level: "INFO", message: "test" },
      ]);

      await api.getRecentLogs(50);

      expect(fetch).toHaveBeenCalledWith(
        "http://localhost:8787/api/logs?limit=50",
        expect.any(Object),
      );
    });

    it("calls without limit when not provided", async () => {
      mockShouldUseRemoteApi.mockReturnValue(false);
      mockInvoke.mockResolvedValue([]);

      await api.getRecentLogs();

      expect(mockInvoke).toHaveBeenCalledWith("get_recent_logs", { limit: undefined });
    });
  });

  describe("clearLogs", () => {
    it("calls local invoke", async () => {
      mockShouldUseRemoteApi.mockReturnValue(false);
      mockInvoke.mockResolvedValue(undefined);

      await api.clearLogs();

      expect(mockInvoke).toHaveBeenCalledWith("clear_logs", undefined);
    });

    it("calls remote fetch with POST", async () => {
      mockShouldUseRemoteApi.mockReturnValue(true);
      mockUseSettingsStore.mockReturnValue({
        ...baseStoreState,
        settings: {
          ...baseSettings,
          engine_mode: "remote_lan",
          lan_enabled: true,
          remote_host: "http://localhost:8787",
          remote_token: "test-token",
        },
      });

      globalThis.fetch = createMockFetch({});

      await api.clearLogs();

      expect(fetch).toHaveBeenCalledWith("http://localhost:8787/api/logs/clear", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer test-token",
        },
        signal: expect.any(AbortSignal),
      });
    });
  });
});
