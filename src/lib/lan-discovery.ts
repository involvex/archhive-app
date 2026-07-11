import type { LanHost } from "./types";

export const LAN_PORT = 8787;

/** Android emulator maps host loopback to this address. */
export const EMULATOR_LAN_HOST: LanHost = {
  name: "Android Emulator (host PC)",
  url: `http://10.0.2.2:${LAN_PORT}`,
  ip: "10.0.2.2",
  port: LAN_PORT,
};

export function isAndroidTauri(): boolean {
  return import.meta.env.TAURI_ENV_PLATFORM === "android";
}

/** True for AVD / sdk_gphone — not physical phones on Wi‑Fi. */
export function isAndroidEmulator(): boolean {
  if (!isAndroidTauri()) return false;
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent.toLowerCase();
  return (
    ua.includes("sdk_gphone") || ua.includes("emulator") || ua.includes("android sdk built for")
  );
}

export function mergeDiscoveredHosts(apiHosts: LanHost[]): LanHost[] {
  const byUrl = new Map<string, LanHost>();
  if (isAndroidEmulator()) {
    byUrl.set(EMULATOR_LAN_HOST.url, EMULATOR_LAN_HOST);
  }
  for (const host of apiHosts) {
    byUrl.set(host.url, host);
  }
  return [...byUrl.values()];
}

export function formatLanUrl(ip: string, port = LAN_PORT): string {
  return `http://${ip}:${port}`;
}
