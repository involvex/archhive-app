import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";
import { initializePlugins } from "./lib/plugins/loader";
import { getRegisteredPlugins } from "./lib/plugins/registry.generated";
import { api } from "./lib/api/client";
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
    if (!isDesktopTauri()) return;
    void api
      .getSettings()
      .then((backend) => {
        useSettingsStore.getState().updateSettings(backend);
      })
      .catch(console.error);
  }, []);
  return null;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BootstrapSettings />
    <RouterProvider router={router} />
  </StrictMode>,
);
