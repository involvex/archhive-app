/** True when running inside a Tauri webview (desktop or mobile). */
export function isTauri(): boolean {
  if (typeof window === "undefined") return false;
  if (import.meta.env.TAURI_ENV_PLATFORM) return true;
  const internals = (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  const legacy = (window as Window & { __TAURI__?: { core?: { invoke?: unknown } } }).__TAURI__;
  return Boolean(internals || legacy?.core?.invoke);
}

/** Android/iOS — uses Tauri platform env or user agent. */
export function isMobileDevice(): boolean {
  const platform = import.meta.env.TAURI_ENV_PLATFORM;
  if (platform === "android" || platform === "ios") return true;
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent.toLowerCase();
  return ua.includes("android") || ua.includes("iphone") || ua.includes("ipad");
}

/** @deprecated Use isMobileDevice — kept for call sites migrating off Tauri-only detection. */
export function isMobileTauri(): boolean {
  return isMobileDevice();
}

export function isDesktopTauri(): boolean {
  return isTauri() && !isMobileDevice();
}
