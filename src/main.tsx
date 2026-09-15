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

function BootstrapSettings() {
  useEffect(() => {
    void bootstrapLanBrowser().then(() => {
      // Sync backend settings on desktop and mobile-tauri (not browser-only LAN UI).
      if (!isTauri()) return;
      void api
        .getSettings()
        .then((backend) => {
          const runtime = getAppRuntime();
          const next = { ...backend };
          // Migrate stale persisted remote_lan on mobile when backend is local/standalone.
          if (
            runtime === "mobile-tauri" &&
            next.engine_mode === "remote_lan" &&
            !next.remote_host?.trim()
          ) {
            next.engine_mode = "local";
          }
          useSettingsStore.getState().updateSettings(next);
        })
        .catch(console.error);
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
      <TrayNavigationListener />
      <RouterProvider router={router} />
      <Toaster position="bottom-right" />
    </AppErrorBoundary>
  </StrictMode>,
);
