import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { resolveAppVersion } from "@/lib/appVersion";
import { ConnectionStatusChip } from "@/components/ConnectionStatusChip";
import {
  buildCookieBookmarklet,
  COOKIE_EXTENSION_URL,
  COOKIE_IMPORT_STEPS,
  importCookiesFromJson,
  importCookiesFromDevToolsTable,
  DEVTOOLS_COOKIE_CONSOLE_SNIPPET,
} from "@/lib/cookies/import";
import { useUnifiedSettings } from "@/hooks/useUnifiedSettings";
import { mergeDiscoveredHosts } from "@/lib/lan-discovery";
import { getPluginSettingsPanels } from "@/lib/plugins/loader";
import { visibleSettingsTabs } from "@/lib/settings/capabilities";
import type {
  CookieSiteInfo,
  DuplicateGroup,
  EngineMode,
  LanHost,
  SiteInfo,
  AppSettings,
} from "@/lib/types";
import { DuplicateGroupCard } from "@/components/DuplicateGroupCard";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import * as Tabs from "@radix-ui/react-tabs";
import * as Switch from "@radix-ui/react-switch";

export const Route = createFileRoute("/settings/")({
  component: SettingsPage,
});

function groupKey(group: DuplicateGroup) {
  return `${group.match_type}:${group.hash}`;
}

const ENGINE_LABELS: Record<EngineMode, string> = {
  local: "Local (Desktop)",
  remote_lan: "Remote LAN",
  standalone: "Standalone (direct URLs only)",
};

function SettingsPage() {
  const {
    runtime,
    caps,
    settings,
    updateSettings,
    hostSettings,
    patchHostSettings,
    saveHostSettings,
    loading,
    error,
  } = useUnifiedSettings();

  const [runtimeLanToken, setRuntimeLanToken] = useState("");
  const displayLanToken = settings.lan_token ?? runtimeLanToken;
  const [testStatus, setTestStatus] = useState("");
  const [sites, setSites] = useState<SiteInfo[]>([]);
  const [cookieSites, setCookieSites] = useState<CookieSiteInfo[]>([]);
  const [selectedSite, setSelectedSite] = useState("");
  const [cookieText, setCookieText] = useState("");
  const [cookieJson, setCookieJson] = useState("");
  const [cookieTable, setCookieTable] = useState("");
  const [cookieStatus, setCookieStatus] = useState("");
  const [duplicates, setDuplicates] = useState<DuplicateGroup[]>([]);
  const [dupSelections, setDupSelections] = useState<Record<string, string>>({});
  const [deleteDupFiles, setDeleteDupFiles] = useState(false);
  const [mergingKey, setMergingKey] = useState<string | null>(null);
  const [dupStatus, setDupStatus] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState("");
  const [scanResult, setScanResult] = useState("");
  const [discoveredHosts, setDiscoveredHosts] = useState<LanHost[]>([]);
  const [discovering, setDiscovering] = useState(false);
  const [discoverStatus, setDiscoverStatus] = useState("");
  const [appVersion, setAppVersion] = useState("…");

  useEffect(() => {
    void resolveAppVersion().then(setAppVersion);
  }, []);

  useEffect(() => {
    if (!caps.lanServer || !settings.lan_enabled) return;
    if (displayLanToken) return;
    void api
      .startLanServer(settings.lan_port)
      .then((r) => {
        setRuntimeLanToken(r.token);
        updateSettings({ lan_token: r.token || undefined });
      })
      .catch(console.error);
  }, [caps.lanServer, settings.lan_enabled, settings.lan_port, displayLanToken, updateSettings]);

  const runLanDiscovery = useCallback(
    async (autoSelect = false) => {
      setDiscovering(true);
      setDiscoverStatus("Searching LAN for ArcHive on port 8787…");
      try {
        const found = mergeDiscoveredHosts(await api.discoverLanHosts(6000));
        setDiscoveredHosts(found);
        if (found.length === 0) {
          setDiscoverStatus(
            "No hosts found. Ensure desktop is running (bun run android:dev starts it on :8787).",
          );
        } else {
          setDiscoverStatus(`Found ${found.length} host(s). Tap one to connect.`);
          if (autoSelect && !settings.remote_host?.trim() && found[0]) {
            updateSettings({ remote_host: found[0].url, remote_token: undefined });
          }
        }
      } catch (e) {
        setDiscoverStatus(e instanceof Error ? e.message : "Discovery failed");
        setDiscoveredHosts(mergeDiscoveredHosts([]));
      } finally {
        setDiscovering(false);
      }
    },
    [settings.remote_host, updateSettings],
  );

  useEffect(() => {
    if (settings.engine_mode !== "remote_lan" || !caps.lanDiscovery) return;
    const timer = window.setTimeout(() => {
      void runLanDiscovery(true);
    }, 0);
    return () => window.clearTimeout(timer);
  }, [settings.engine_mode, caps.lanDiscovery, runLanDiscovery]);

  useEffect(() => {
    if (!caps.libraryScanLocal) return;
    let unlisten: (() => void) | undefined;
    void api
      .subscribeScanProgress((p) => {
        setScanProgress(`Scanned ${p.scanned} — added ${p.added}, updated ${p.updated}`);
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, [caps.libraryScanLocal]);

  async function refreshCookies() {
    try {
      setCookieSites(await api.listCookieSites());
    } catch {
      /* remote not configured yet */
    }
  }

  useEffect(() => {
    void api.listSites().then(setSites).catch(console.error);
    void api
      .listCookieSites()
      .then(setCookieSites)
      .catch(() => {});
  }, []);

  async function copyLanToken() {
    if (!displayLanToken) return;
    await navigator.clipboard.writeText(displayLanToken);
    setTestStatus("LAN token copied — paste into mobile Settings → Engine → Remote token.");
  }

  async function copyWebLink() {
    const port = settings.lan_port;
    const token = displayLanToken?.trim();
    let base = `http://127.0.0.1:${port}`;
    if (settings.lan_enabled) {
      try {
        const headers: HeadersInit = token ? { Authorization: `Bearer ${token}` } : {};
        const res = await fetch(`http://127.0.0.1:${port}/api/health`, { headers });
        if (res.ok) {
          const health = (await res.json()) as { lan_url?: string };
          if (health.lan_url) base = health.lan_url;
        }
      } catch {
        // fall back to localhost
      }
    }
    const url = token ? `${base}/?token=${encodeURIComponent(token)}` : `${base}/`;
    await navigator.clipboard.writeText(url);
    setTestStatus(`Web UI link copied. Open on phone: ${base}/ — folder browser at ${base}/files`);
  }

  async function regenerateLanToken() {
    const result = await api.regenerateLanServer(settings.lan_port);
    setRuntimeLanToken(result.token);
    updateSettings({ lan_enabled: true, lan_token: result.token || undefined });
    setTestStatus("LAN token regenerated. Update mobile clients with the new token.");
  }

  async function enableLan() {
    const result = await api.startLanServer(settings.lan_port);
    setRuntimeLanToken(result.token);
    updateSettings({ lan_enabled: true, lan_token: result.token });
  }

  function selectDiscoveredHost(host: LanHost) {
    updateSettings({ remote_host: host.url, remote_token: undefined });
    setDiscoverStatus(`Selected ${host.url} (no token)`);
  }

  async function testRemote() {
    try {
      const health = await api.testRemoteConnection(
        settings.remote_host || "",
        settings.remote_token,
      );
      const authNote =
        health.auth_required === false ? " — no token required" : " — token required";
      setTestStatus(`Connected to ArcHive v${health.version}${authNote}`);
    } catch {
      setTestStatus("Connection failed");
    }
  }

  async function runLibraryScan() {
    setScanning(true);
    setScanProgress("Starting scan…");
    setScanResult("");
    try {
      const result = await api.scanLibrary();
      setScanResult(`Done — added ${result.added}, updated ${result.updated}`);
      setScanProgress("");
    } catch (e) {
      setScanResult(e instanceof Error ? e.message : "Scan failed");
    } finally {
      setScanning(false);
    }
  }

  async function saveCookies() {
    if (!selectedSite || !cookieText.trim()) return;
    await api.saveSiteCookies(selectedSite, cookieText);
    setCookieText("");
    setCookieStatus("Cookies saved.");
    await refreshCookies();
  }

  async function importDevToolsCookies() {
    if (!selectedSite || !cookieTable.trim()) return;
    try {
      const netscape = importCookiesFromDevToolsTable(cookieTable, selectedSite);
      setCookieText(netscape);
      await api.saveSiteCookies(selectedSite, netscape);
      setCookieTable("");
      setCookieStatus(`Imported ${netscape.split("\n").length - 4} cookies from DevTools table.`);
      await refreshCookies();
    } catch (e) {
      setCookieStatus(e instanceof Error ? e.message : "Import failed");
    }
  }

  async function copyConsoleSnippet() {
    await navigator.clipboard.writeText(DEVTOOLS_COOKIE_CONSOLE_SNIPPET);
    setCookieStatus(
      "Console snippet copied. Run it on the logged-in site tab, then paste JSON below. Note: httpOnly cookies need DevTools table import.",
    );
  }

  async function importJsonCookies() {
    if (!selectedSite || !cookieJson.trim()) return;
    try {
      const netscape = importCookiesFromJson(cookieJson, selectedSite);
      setCookieText(netscape);
      await api.saveSiteCookies(selectedSite, netscape);
      setCookieJson("");
      setCookieStatus(`Imported ${netscape.split("\n").length - 4} cookie lines.`);
      await refreshCookies();
    } catch (e) {
      setCookieStatus(e instanceof Error ? e.message : "Import failed");
    }
  }

  function copyBookmarklet() {
    const site = cookieSitesList.find((s) => s.id === selectedSite);
    if (!site) {
      setCookieStatus("Select a site first.");
      return;
    }
    const bookmarklet = buildCookieBookmarklet(site.base_url);
    void navigator.clipboard.writeText(bookmarklet);
    setCookieStatus("Bookmarklet copied. Create a browser bookmark with the pasted URL.");
  }

  async function removeCookies(siteId: string) {
    await api.deleteSiteCookies(siteId);
    await refreshCookies();
  }

  async function scanDuplicates() {
    const groups = await api.findDuplicates();
    setDuplicates(groups);
    const defaults: Record<string, string> = {};
    for (const group of groups) {
      defaults[groupKey(group)] = group.scenes[0]?.id ?? "";
    }
    setDupSelections(defaults);
  }

  async function mergeGroup(group: DuplicateGroup) {
    const key = groupKey(group);
    const keepId = dupSelections[key] ?? group.scenes[0]?.id;
    if (!keepId) return;
    const removeIds = group.scenes.map((s) => s.id).filter((id) => id !== keepId);
    if (removeIds.length === 0) return;

    setMergingKey(key);
    try {
      const result = await api.mergeDuplicates(keepId, removeIds, deleteDupFiles);
      setDuplicates((prev) => prev.filter((g) => groupKey(g) !== key));
      setDupStatus(`Removed ${result.removed} duplicate scene(s).`);
    } finally {
      setMergingKey(null);
    }
  }

  const cookieSitesList = sites.filter((s) => s.requires_cookies);
  const canScan = caps.libraryScanLocal || caps.libraryScanRemote;
  const pluginPanels = getPluginSettingsPanels();
  const pluginPanelsByTab = (tab: string) => pluginPanels.filter((p) => p.tab === tab);
  const settingsTabs = visibleSettingsTabs(caps);

  async function persistTraySettings(partial: Partial<AppSettings>) {
    if (!hostSettings) return;
    const merged = { ...hostSettings, ...partial };
    patchHostSettings(partial);
    await api.saveSettings(merged);
    updateSettings(partial);
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <h2 className="text-2xl font-bold">Settings</h2>

      {caps.showBrowserBanner && (
        <Card className="border-[var(--color-primary)]">
          <CardContent className="p-4 text-sm">
            Browser mode — connect via <strong>Engine → Remote LAN</strong> (
            <code>http://&lt;pc-ip&gt;:8787</code> + token). LAN server runs only in the desktop
            app.
          </CardContent>
        </Card>
      )}

      {error && <p className="text-sm text-yellow-400">{error}</p>}

      {loading && <p className="text-sm text-[var(--color-muted-foreground)]">Loading settings…</p>}

      <Tabs.Root defaultValue="engine">
        <Tabs.List className="flex flex-wrap gap-2 border-b border-[var(--color-border)] pb-2">
          {settingsTabs.map((tab) => (
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
              {caps.engineModes.map((mode) => (
                <label key={mode} className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="engine"
                    checked={settings.engine_mode === mode}
                    onChange={() => updateSettings({ engine_mode: mode })}
                  />
                  {ENGINE_LABELS[mode]}
                  {mode === "remote_lan" && runtime !== "desktop-tauri" && " (recommended)"}
                </label>
              ))}
              {caps.lanServer && settings.lan_enabled && displayLanToken && (
                <Card className="border-[var(--color-primary)]">
                  <CardContent className="p-4 text-sm space-y-2">
                    <p>
                      <strong>Mobile LAN token</strong> (copy to Android Settings → Engine):
                    </p>
                    <code className="block break-all text-xs">{displayLanToken}</code>
                    <Button variant="outline" size="sm" onClick={() => void copyLanToken()}>
                      Copy token
                    </Button>
                  </CardContent>
                </Card>
              )}
              {settings.engine_mode === "remote_lan" && (
                <div className="space-y-2 pt-2">
                  <ConnectionStatusChip />
                  {!caps.lanServer && (
                    <p className="rounded-md border border-[var(--color-border)] bg-[var(--color-muted)] p-3 text-xs">
                      Desktop LAN runs on port <strong>8787</strong> (not 1420).{" "}
                      <code>bun run android:dev</code> auto-starts it; token is optional in dev
                      mode.
                    </p>
                  )}
                  {caps.lanDiscovery && (
                    <div className="space-y-2 rounded-md border border-[var(--color-border)] p-3">
                      <div className="flex items-center justify-between gap-2">
                        <p className="text-xs font-medium">LAN discovery (mDNS)</p>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => void runLanDiscovery()}
                          disabled={discovering}
                        >
                          {discovering ? "Searching…" : "Scan again"}
                        </Button>
                      </div>
                      {discoveredHosts.length > 0 ? (
                        <ul className="space-y-1">
                          {discoveredHosts.map((host) => (
                            <li key={host.url}>
                              <button
                                type="button"
                                className="w-full rounded-md border border-[var(--color-border)] px-3 py-2 text-left text-xs hover:bg-[var(--color-muted)]"
                                onClick={() => selectDiscoveredHost(host)}
                              >
                                <span className="font-medium">{host.name}</span>
                                <span className="block text-[var(--color-muted-foreground)]">
                                  {host.url}
                                </span>
                              </button>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="text-xs text-[var(--color-muted-foreground)]">
                          {discovering
                            ? "Searching…"
                            : "No hosts yet. Start desktop ArcHive with LAN enabled."}
                        </p>
                      )}
                      {discoverStatus && (
                        <p className="text-xs text-[var(--color-muted-foreground)]">
                          {discoverStatus}
                        </p>
                      )}
                    </div>
                  )}
                  <Input
                    placeholder="http://192.168.178.69:8787"
                    value={settings.remote_host || ""}
                    onChange={(e) => updateSettings({ remote_host: e.target.value })}
                  />
                  <Input
                    placeholder="API token (optional if desktop LAN has no token)"
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
          {pluginPanelsByTab("engine").map((panel) => (
            <Card key={panel.id}>
              <CardHeader>
                <CardTitle className="text-base">{panel.title}</CardTitle>
              </CardHeader>
              <CardContent>{panel.render()}</CardContent>
            </Card>
          ))}
        </Tabs.Content>

        <Tabs.Content value="library" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Library</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {!caps.libraryPathEditable && (
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Library path is managed on the desktop host. Scan runs on the host when connected.
                </p>
              )}
              <div>
                <label className="text-xs text-[var(--color-muted-foreground)]">Library path</label>
                <Input
                  value={hostSettings?.library_path || ""}
                  readOnly={!caps.libraryPathEditable}
                  onChange={(e) => patchHostSettings({ library_path: e.target.value })}
                />
              </div>
              <div>
                <label className="text-xs text-[var(--color-muted-foreground)]">
                  Naming template
                </label>
                <Input
                  value={hostSettings?.naming_template || ""}
                  readOnly={!caps.libraryPathEditable}
                  onChange={(e) => patchHostSettings({ naming_template: e.target.value })}
                />
              </div>
              <div>
                <label className="text-xs text-[var(--color-muted-foreground)]">
                  phash Hamming threshold (0 = exact, 10 = near-duplicate)
                </label>
                <Input
                  type="number"
                  min={0}
                  max={32}
                  value={hostSettings?.phash_threshold ?? 10}
                  readOnly={!caps.libraryPathEditable}
                  onChange={(e) => patchHostSettings({ phash_threshold: Number(e.target.value) })}
                />
              </div>
              {caps.libraryPathEditable && (
                <Button onClick={() => void saveHostSettings()} disabled={!hostSettings}>
                  Save
                </Button>
              )}
              {canScan && (
                <>
                  <Button
                    variant="outline"
                    onClick={() => void runLibraryScan()}
                    disabled={scanning}
                  >
                    {scanning
                      ? "Scanning…"
                      : caps.libraryScanRemote
                        ? "Scan on host"
                        : "Scan Library"}
                  </Button>
                  {scanProgress && (
                    <p className="text-xs text-[var(--color-muted-foreground)]">{scanProgress}</p>
                  )}
                  {scanResult && (
                    <p className="text-xs text-[var(--color-muted-foreground)]">{scanResult}</p>
                  )}
                </>
              )}
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
                Import via Cookie-Editor JSON, DevTools cookie table (no extension), or Netscape
                format.
              </p>
              <ol className="list-decimal space-y-1 pl-5 text-xs text-[var(--color-muted-foreground)]">
                {COOKIE_IMPORT_STEPS.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ol>
              <a
                href={COOKIE_EXTENSION_URL}
                target="_blank"
                rel="noreferrer"
                className="text-xs text-[var(--color-primary)] underline"
              >
                Get Cookie-Editor extension
              </a>
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
              <div className="flex flex-wrap gap-2">
                <Button variant="outline" onClick={copyBookmarklet} disabled={!selectedSite}>
                  Copy login bookmarklet
                </Button>
                <Button variant="outline" onClick={() => void copyConsoleSnippet()}>
                  Copy console snippet
                </Button>
              </div>
              <textarea
                className="min-h-20 w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-xs font-mono"
                placeholder="Paste DevTools Application → Cookies table rows (Name, Value, Domain…)"
                value={cookieTable}
                onChange={(e) => setCookieTable(e.target.value)}
              />
              <Button onClick={() => void importDevToolsCookies()} disabled={!selectedSite}>
                Import DevTools table
              </Button>
              <textarea
                className="min-h-24 w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-xs font-mono"
                placeholder='[{"name":"sess","value":"...","domain":".example.com"}]'
                value={cookieJson}
                onChange={(e) => setCookieJson(e.target.value)}
              />
              <Button onClick={() => void importJsonCookies()} disabled={!selectedSite}>
                Import JSON
              </Button>
              <textarea
                className="min-h-24 w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-xs font-mono"
                placeholder="# Netscape HTTP Cookie File"
                value={cookieText}
                onChange={(e) => setCookieText(e.target.value)}
              />
              <Button onClick={() => void saveCookies()} disabled={!selectedSite}>
                Save Netscape Cookies
              </Button>
              {cookieStatus && (
                <p className="text-xs text-[var(--color-muted-foreground)]">{cookieStatus}</p>
              )}
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
                Groups scenes by perceptual hash (Hamming ≤ threshold in Library settings) or exact
                oshash.
              </p>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={deleteDupFiles}
                  onChange={(e) => setDeleteDupFiles(e.target.checked)}
                />
                Delete duplicate files from disk
              </label>
              <Button variant="outline" onClick={() => void scanDuplicates()}>
                Scan for Duplicates
              </Button>
              {dupStatus && (
                <p className="text-xs text-[var(--color-muted-foreground)]">{dupStatus}</p>
              )}
              {duplicates.length === 0 ? (
                <p className="text-sm text-[var(--color-muted-foreground)]">
                  No duplicate groups found.
                </p>
              ) : (
                <ul className="space-y-3 text-sm">
                  {duplicates.map((group) => {
                    const key = groupKey(group);
                    return (
                      <DuplicateGroupCard
                        key={key}
                        group={group}
                        groupKey={key}
                        selectedId={dupSelections[key]}
                        deleteFiles={deleteDupFiles}
                        merging={mergingKey === key}
                        onSelect={(sceneId) =>
                          setDupSelections((prev) => ({ ...prev, [key]: sceneId }))
                        }
                        onMerge={() => void mergeGroup(group)}
                      />
                    );
                  })}
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
              {!caps.lanServer ? (
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  The LAN server runs only in the desktop app. Open ArcHive on your PC → Settings →
                  LAN to enable API on port 8787.
                </p>
              ) : (
                <>
                  <p className="text-xs text-[var(--color-muted-foreground)]">
                    Serves the web UI and API on your LAN. Open{" "}
                    <code>http://&lt;pc-ip&gt;:{settings.lan_port}/</code> in a phone browser, or
                    use Remote LAN in the mobile app. Folder browser: <code>/files</code>.
                  </p>
                  <div className="flex items-center gap-3">
                    <Switch.Root
                      checked={settings.lan_enabled}
                      onCheckedChange={(checked) => {
                        if (checked) void enableLan();
                        else
                          void api
                            .stopLanServer()
                            .then(() => updateSettings({ lan_enabled: false }));
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
                  {displayLanToken && (
                    <div className="space-y-2">
                      <p className="text-xs break-all">
                        API Token: <code>{displayLanToken}</code>
                      </p>
                      <div className="flex flex-wrap gap-2">
                        <Button variant="outline" size="sm" onClick={() => void copyLanToken()}>
                          Copy token
                        </Button>
                        <Button variant="outline" size="sm" onClick={() => void copyWebLink()}>
                          Copy web link
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => void regenerateLanToken()}
                        >
                          Regenerate
                        </Button>
                      </div>
                      <p className="text-xs text-[var(--color-muted-foreground)]">
                        Mobile app: Settings → Engine → paste token. Phone browser: use Copy web
                        link (includes token in URL when required).
                      </p>
                    </div>
                  )}
                </>
              )}
            </CardContent>
          </Card>
        </Tabs.Content>

        <Tabs.Content value="desktop" className="mt-4 space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">System Tray</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-xs text-[var(--color-muted-foreground)]">
                ArcHive stays in the system tray when hidden. Left-click the tray icon or use the
                hotkey to show or hide the window. Tray menu includes quick access to Settings.
              </p>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-medium">Close to tray</p>
                  <p className="text-xs text-[var(--color-muted-foreground)]">
                    When enabled, the close button hides the app instead of quitting.
                  </p>
                </div>
                <Switch.Root
                  checked={hostSettings?.close_to_tray ?? true}
                  onCheckedChange={(checked) =>
                    void persistTraySettings({ close_to_tray: checked })
                  }
                  className="h-5 w-9 shrink-0 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
                >
                  <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
                </Switch.Root>
              </div>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-medium">Minimize to tray</p>
                  <p className="text-xs text-[var(--color-muted-foreground)]">
                    When enabled, minimizing the window sends it to the tray.
                  </p>
                </div>
                <Switch.Root
                  checked={hostSettings?.minimize_to_tray ?? true}
                  onCheckedChange={(checked) =>
                    void persistTraySettings({ minimize_to_tray: checked })
                  }
                  className="h-5 w-9 shrink-0 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
                >
                  <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
                </Switch.Root>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium" htmlFor="tray-hotkey">
                  Show / hide hotkey
                </label>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Global shortcut to toggle the window. Leave empty to disable. Example:{" "}
                  <code>Ctrl+Shift+A</code>
                </p>
                <Input
                  id="tray-hotkey"
                  value={hostSettings?.tray_hotkey ?? ""}
                  placeholder="Ctrl+Shift+A"
                  onChange={(e) => patchHostSettings({ tray_hotkey: e.target.value || undefined })}
                  onBlur={() =>
                    void persistTraySettings({
                      tray_hotkey: hostSettings?.tray_hotkey?.trim() || undefined,
                    })
                  }
                />
              </div>
            </CardContent>
          </Card>
        </Tabs.Content>
      </Tabs.Root>

      <p className="text-xs text-[var(--color-muted-foreground)]">
        ArcHive v{appVersion} · {runtime}
      </p>
    </div>
  );
}
