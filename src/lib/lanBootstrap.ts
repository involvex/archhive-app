import { useSettingsStore } from "./stores/settings";
import { isTauri } from "./tauri";

/** When the SPA is opened from the desktop LAN server, wire API to same origin. */
export async function bootstrapLanBrowser(): Promise<void> {
  if (isTauri()) return;

  const origin = window.location.origin;
  const params = new URLSearchParams(window.location.search);
  const urlToken = params.get("token")?.trim();

  if (urlToken) {
    useSettingsStore.getState().setRemoteToken(urlToken);
    params.delete("token");
    const next = `${window.location.pathname}${params.toString() ? `?${params}` : ""}${window.location.hash}`;
    window.history.replaceState({}, "", next);
  }

  try {
    const res = await fetch(`${origin}/api/health`);
    if (!res.ok) return;
    const health = (await res.json()) as {
      auth_required?: boolean;
      lan_url?: string;
    };

    const { settings, updateSettings } = useSettingsStore.getState();
    const host = health.lan_url ?? origin;
    if (!settings.remote_host?.trim() || settings.remote_host === origin) {
      updateSettings({ remote_host: host, engine_mode: "remote_lan" });
    }

    if (health.auth_required === false) {
      updateSettings({ remote_token: undefined });
    }
  } catch {
    // Not served from ArcHive LAN — user may be on Vite dev port 1420.
  }
}
