import type { SiteInfo } from "../types";
import type {
  ArcHivePlugin,
  PluginContext,
  PluginNavItem,
  PluginRoute,
  PluginSettingsPanel,
} from "./types";

const browseSites: SiteInfo[] = [];
const settingsPanels: PluginSettingsPanel[] = [];
const navItems: PluginNavItem[] = [];
const routes: PluginRoute[] = [];

function createContext(): PluginContext {
  return {
    addBrowseSite(site) {
      browseSites.push(site);
    },
    addSettingsPanel(panel) {
      settingsPanels.push(panel);
    },
    addNavItem(item) {
      navItems.push(item);
    },
    addRoute(route) {
      routes.push(route);
    },
  };
}

let initialized = false;

export function initializePlugins(plugins: ArcHivePlugin[]): void {
  if (initialized) return;
  initialized = true;
  const ctx = createContext();
  for (const plugin of plugins) {
    plugin.register(ctx);
  }
}

export function getPluginBrowseSites(): SiteInfo[] {
  return browseSites;
}

export function getPluginSettingsPanels(): PluginSettingsPanel[] {
  return settingsPanels;
}

export function getPluginNavItems(): PluginNavItem[] {
  return navItems;
}

export function getPluginRoutes(): PluginRoute[] {
  return routes;
}

export function getPluginRoute(path: string): PluginRoute | undefined {
  return routes.find((r) => r.path === path);
}
