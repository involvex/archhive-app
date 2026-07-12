/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_APP_VERSION?: string;
  readonly TAURI_ENV_PLATFORM?: "windows" | "macos" | "linux" | "android" | "ios";
  readonly TAURI_ENV_DEBUG?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
