import type { ReactNode } from "react";
import type { SiteInfo } from "../types";
import type { SettingsTab } from "../settings/capabilities";

export interface PluginSettingsPanel {
  id: string;
  title: string;
  tab: SettingsTab;
  render: () => ReactNode;
}

export interface PluginNavItem {
  to: string;
  label: string;
}

export interface PluginRoute {
  path: string;
  title: string;
  component: () => ReactNode;
}

export interface PluginContext {
  addBrowseSite: (site: SiteInfo) => void;
  addSettingsPanel: (panel: PluginSettingsPanel) => void;
  addNavItem: (item: PluginNavItem) => void;
  addRoute: (route: PluginRoute) => void;
}

export interface ArcHivePlugin {
  id: string;
  register: (ctx: PluginContext) => void;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  entry: string;
  browseSites?: SiteInfo[];
  routes?: { path: string; title: string }[];
  settingsPanels?: { id: string; title: string; tab: SettingsTab }[];
}
