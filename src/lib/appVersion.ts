import { getVersion } from "@tauri-apps/api/app";

/** Sync version baked in at Vite build from package.json. */
export const APP_VERSION = import.meta.env.VITE_APP_VERSION as string;

/** Best available app version (Tauri runtime or Vite build stamp). */
export async function resolveAppVersion(): Promise<string> {
  if (APP_VERSION) return APP_VERSION;
  try {
    return await getVersion();
  } catch {
    return "dev";
  }
}
