import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "react-hot-toast";
import { routeTree } from "./routeTree.gen";
import { initializePlugins } from "./lib/plugins/loader";
import { getRegisteredPlugins } from "./lib/plugins/registry.generated";
import { api } from "./lib/api/client";
import { bootstrapLanBrowser } from "./lib/lanBootstrap";
import { useSettingsStore } from "./lib/stores/settings";
import { isDesktopTauri, isTauri } from "./lib/tauri";
import { getAppRuntime } from "./lib/runtime";
import { AppErrorBoundary } from "./components/AppErrorBoundary";
import { useDownloadNotifications } from "@/lib/hooks/useDownloadNotifications";
import { FirstRunWizardGate } from "./components/FirstRunWizard";
import { initAmoled } from "./lib/amoled";
import "./styles/globals.css";

initAmoled();
initializePlugins(getRegisteredPlugins());

const router = createRouter({
  routeTree,
  basepath: __GHPAGES_BASEPATH__,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

/**
 * #69: weekly desktop binary update check. Runs in the background after
 * settings load; toasts only when an update is actually available.
 */
async function maybeAutoCheckBinaries(
  settings: ReturnType<typeof useSettingsStore.getState>["settings"],
): Promise<void> {
  try {
    if (getAppRuntime() !== "desktop-tauri") return;
    if (settings.auto_check_binaries === false) return;
    const last = settings.last_binary_check ?? 0;
    if (Date.now() / 1000 - last < 7 * 24 * 3600) return;
    const [current, latest] = await Promise.all([
      api.binaryVersions().catch(() => null),
      api.checkBinaryUpdates().catch(() => null),
    ]);
    const stamp = Math.floor(Date.now() / 1000);
    const merged = { ...useSettingsStore.getState().settings, last_binary_check: stamp };
    useSettingsStore.getState().updateSettings({ last_binary_check: stamp });
    void api.saveSettings(merged).catch(() => {});
    if (!current || !latest) return;
    const updates: string[] = [];
    if (latest.ytdlp_latest && latest.ytdlp_latest !== current.ytdlp_version) {
      updates.push(`yt-dlp v${latest.ytdlp_latest}`);
    }
    if (latest.gallery_dl_latest && latest.gallery_dl_latest !== current.gallery_dl_version) {
      updates.push(`gallery-dl v${latest.gallery_dl_latest}`);
    }
    if (updates.length > 0) {
      toast(`Binary updates available: ${updates.join(", ")} — see Settings → Library`, {
        duration: 8000,
      });
    }
  } catch {
    /* background check must never break startup */
  }
}

function BootstrapSettings() {
  useEffect(() => {
    void bootstrapLanBrowser().then(() => {
      // Sync backend settings on desktop and mobile-tauri (not browser-only LAN UI).
      if (!isTauri()) return;
      const runtime = getAppRuntime();
      // Mobile: always read device SQLite so engine mode / cookies don't flip from LAN.
      const load = runtime === "mobile-tauri" ? api.getDeviceSettings() : api.getSettings();
      void load
        .then((backend) => {
          const next = { ...backend };
          // Migrate stale persisted remote_lan on mobile when no host is configured.
          if (
            runtime === "mobile-tauri" &&
            next.engine_mode === "remote_lan" &&
            !next.remote_host?.trim()
          ) {
            next.engine_mode = "local";
            void api.saveDeviceSettings(next).catch(() => {});
          }
          useSettingsStore.getState().updateSettings(next);
          useSettingsStore.getState().setHydrated(true);
          void maybeAutoCheckBinaries(next);
        })
        .catch((e) => {
          console.error(e);
          // Still mark hydrated so the wizard gate can fall back to local settings.
          useSettingsStore.getState().setHydrated(true);
        });
    });
  }, []);
  return null;
}

function TrayNavigationListener() {
  useEffect(() => {
    if (!isDesktopTauri()) return;
    let unlisten: (() => void) | undefined;
    void listen<string>("app-navigate", (event) => {
      const path = event.payload;
      if (path) void router.navigate({ to: path });
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);
  return null;
}

function DownloadNotifications() {
  useDownloadNotifications(toast);
  return null;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppErrorBoundary>
      <DownloadNotifications />
      <BootstrapSettings />
      <FirstRunWizardGate />
      <TrayNavigationListener />
      <RouterProvider router={router} />
      <Toaster position="bottom-right" />
    </AppErrorBoundary>
  </StrictMode>,
);
