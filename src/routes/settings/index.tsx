import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type {
  AppSettings,
  CookieSiteInfo,
  DuplicateGroup,
  EngineMode,
  SiteInfo,
} from "@/lib/types";
import { useSettingsStore } from "@/lib/stores/settings";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import * as Tabs from "@radix-ui/react-tabs";
import * as Switch from "@radix-ui/react-switch";

export const Route = createFileRoute("/settings/")({
  component: SettingsPage,
});

function SettingsPage() {
  const { settings, updateSettings } = useSettingsStore();
  const [backendSettings, setBackendSettings] = useState<AppSettings | null>(null);
  const [lanToken, setLanToken] = useState("");
  const [testStatus, setTestStatus] = useState("");
  const [sites, setSites] = useState<SiteInfo[]>([]);
  const [cookieSites, setCookieSites] = useState<CookieSiteInfo[]>([]);
  const [selectedSite, setSelectedSite] = useState("");
  const [cookieText, setCookieText] = useState("");
  const [duplicates, setDuplicates] = useState<DuplicateGroup[]>([]);

  useEffect(() => {
    void api.getSettings().then(setBackendSettings).catch(console.error);
    void api.listSites().then(setSites).catch(console.error);
    void refreshCookies().catch(console.error);
  }, []);

  async function refreshCookies() {
    setCookieSites(await api.listCookieSites());
  }

  async function saveBackend() {
    if (backendSettings) {
      await api.saveSettings(backendSettings);
    }
  }

  async function enableLan() {
    const result = await api.startLanServer(settings.lan_port);
    setLanToken(result.token);
    updateSettings({ lan_enabled: true, lan_token: result.token });
  }

  async function testRemote() {
    try {
      const health = await api.testRemoteConnection(
        settings.remote_host || "",
        settings.remote_token,
      );
      setTestStatus(`Connected: ${health.version}`);
    } catch {
      setTestStatus("Connection failed");
    }
  }

  async function saveCookies() {
    if (!selectedSite || !cookieText.trim()) return;
    await api.saveSiteCookies(selectedSite, cookieText);
    setCookieText("");
    await refreshCookies();
  }

  async function removeCookies(siteId: string) {
    await api.deleteSiteCookies(siteId);
    await refreshCookies();
  }

  async function scanDuplicates() {
    setDuplicates(await api.findDuplicates());
  }

  const engineModes: { value: EngineMode; label: string }[] = [
    { value: "local", label: "Local (Desktop)" },
    { value: "remote_lan", label: "Remote LAN" },
    { value: "standalone", label: "Standalone Mobile" },
  ];

  const cookieSitesList = sites.filter((s) => s.requires_cookies);

  return (
    <div className="space-y-6 max-w-2xl">
      <h2 className="text-2xl font-bold">Settings</h2>

      <Tabs.Root defaultValue="engine">
        <Tabs.List className="flex flex-wrap gap-2 border-b border-[var(--color-border)] pb-2">
          {["engine", "library", "cookies", "duplicates", "lan"].map((tab) => (
            <Tabs.Trigger
              key={tab}
              value={tab}
              className="rounded-md px-3 py-1.5 text-sm capitalize data-[state=active]:bg-[var(--color-primary)] data-[state=active]:text-[var(--color-primary-foreground)]"
            >
              {tab}
            </Tabs.Trigger>
          ))}
        </Tabs.List>

        <Tabs.Content value="engine" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Engine Mode</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {engineModes.map((mode) => (
                <label key={mode.value} className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="engine"
                    checked={settings.engine_mode === mode.value}
                    onChange={() => updateSettings({ engine_mode: mode.value })}
                  />
                  {mode.label}
                </label>
              ))}
              {settings.engine_mode === "remote_lan" && (
                <div className="space-y-2 pt-2">
                  <Input
                    placeholder="http://192.168.1.10:8787"
                    value={settings.remote_host || ""}
                    onChange={(e) => updateSettings({ remote_host: e.target.value })}
                  />
                  <Input
                    placeholder="API token"
                    type="password"
                    value={settings.remote_token || ""}
                    onChange={(e) => updateSettings({ remote_token: e.target.value })}
                  />
                  <Button variant="outline" onClick={() => void testRemote()}>
                    Test Connection
                  </Button>
                  {testStatus && (
                    <p className="text-xs text-[var(--color-muted-foreground)]">{testStatus}</p>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </Tabs.Content>

        <Tabs.Content value="library" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Library</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div>
                <label className="text-xs text-[var(--color-muted-foreground)]">Library path</label>
                <Input
                  value={backendSettings?.library_path || ""}
                  onChange={(e) =>
                    setBackendSettings((s) => s && { ...s, library_path: e.target.value })
                  }
                />
              </div>
              <div>
                <label className="text-xs text-[var(--color-muted-foreground)]">
                  Naming template
                </label>
                <Input
                  value={backendSettings?.naming_template || ""}
                  onChange={(e) =>
                    setBackendSettings((s) => s && { ...s, naming_template: e.target.value })
                  }
                />
              </div>
              <Button onClick={() => void saveBackend()}>Save</Button>
              <Button variant="outline" onClick={() => void api.scanLibrary()}>
                Scan Library
              </Button>
            </CardContent>
          </Card>
        </Tabs.Content>

        <Tabs.Content value="cookies" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Cookie Vault</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-xs text-[var(--color-muted-foreground)]">
                Paste Netscape-format cookies per site. Stored encrypted at rest; used for browse
                and yt-dlp downloads.
              </p>
              <select
                className="w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-sm"
                value={selectedSite}
                onChange={(e) => setSelectedSite(e.target.value)}
              >
                <option value="">Select site…</option>
                {cookieSitesList.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.display_name}
                  </option>
                ))}
              </select>
              <textarea
                className="min-h-32 w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-xs font-mono"
                placeholder="# Netscape HTTP Cookie File"
                value={cookieText}
                onChange={(e) => setCookieText(e.target.value)}
              />
              <Button onClick={() => void saveCookies()} disabled={!selectedSite}>
                Save Cookies
              </Button>
              {cookieSites.length > 0 && (
                <ul className="space-y-2 text-sm">
                  {cookieSites.map((c) => (
                    <li key={c.site_id} className="flex items-center justify-between gap-2">
                      <span>
                        {c.site_id}{" "}
                        <span className="text-xs text-[var(--color-muted-foreground)]">
                          ({c.updated_at})
                        </span>
                      </span>
                      <Button variant="outline" onClick={() => void removeCookies(c.site_id)}>
                        Remove
                      </Button>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </Tabs.Content>

        <Tabs.Content value="duplicates" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Duplicate Finder</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-xs text-[var(--color-muted-foreground)]">
                Groups scenes by perceptual hash (thumbnail) or oshash (file content).
              </p>
              <Button variant="outline" onClick={() => void scanDuplicates()}>
                Scan for Duplicates
              </Button>
              {duplicates.length === 0 ? (
                <p className="text-sm text-[var(--color-muted-foreground)]">
                  No duplicate groups found.
                </p>
              ) : (
                <ul className="space-y-3 text-sm">
                  {duplicates.map((group) => (
                    <li
                      key={`${group.match_type}-${group.hash}`}
                      className="rounded-md border border-[var(--color-border)] p-3"
                    >
                      <p className="font-medium">
                        {group.match_type} · {group.scenes.length} scenes
                      </p>
                      <ul className="mt-1 list-disc pl-5 text-xs text-[var(--color-muted-foreground)]">
                        {group.scenes.map((scene) => (
                          <li key={scene.id}>{scene.title}</li>
                        ))}
                      </ul>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </Tabs.Content>

        <Tabs.Content value="lan" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">LAN Web Server</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-xs text-[var(--color-muted-foreground)]">
                Serves API and built UI from <code>dist/</code> when available.
              </p>
              <div className="flex items-center gap-3">
                <Switch.Root
                  checked={settings.lan_enabled}
                  onCheckedChange={(checked) => {
                    if (checked) void enableLan();
                    else
                      void api.stopLanServer().then(() => updateSettings({ lan_enabled: false }));
                  }}
                  className="h-5 w-9 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
                >
                  <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
                </Switch.Root>
                <span className="text-sm">Enable LAN server</span>
              </div>
              <Input
                type="number"
                value={settings.lan_port}
                onChange={(e) => updateSettings({ lan_port: Number(e.target.value) })}
              />
              {lanToken && (
                <p className="text-xs break-all">
                  API Token: <code>{lanToken}</code>
                </p>
              )}
            </CardContent>
          </Card>
        </Tabs.Content>
      </Tabs.Root>
    </div>
  );
}
