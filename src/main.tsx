import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { routeTree } from "./routeTree.gen";
import { initializePlugins } from "./lib/plugins/loader";
import { getRegisteredPlugins } from "./lib/plugins/registry.generated";
import { api } from "./lib/api/client";
import { bootstrapLanBrowser } from "./lib/lanBootstrap";
import { useSettingsStore } from "./lib/stores/settings";
import { isDesktopTauri } from "./lib/tauri";
import "./styles/globals.css";

initializePlugins(getRegisteredPlugins());

const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

function BootstrapSettings() {
  useEffect(() => {
    void bootstrapLanBrowser().then(() => {
      if (!isDesktopTauri()) return;
      void api
        .getSettings()
        .then((backend) => {
          useSettingsStore.getState().updateSettings(backend);
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

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BootstrapSettings />
    <TrayNavigationListener />
    <RouterProvider router={router} />
  </StrictMode>,
);
