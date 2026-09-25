import { useEffect, useState } from "react";
import { WifiOff } from "lucide-react";
import { useSettingsStore } from "@/lib/stores/settings";
import { getAppRuntime } from "@/lib/runtime";

/**
 * Global offline indicator. On mobile with on-device files the library keeps
 * working without a connection — this banner makes that explicit instead of
 * leaving users guessing why browse/search fail while playback works.
 * Hidden on desktop (always-online assumption) and while online.
 */
export function OfflineBanner() {
  const [online, setOnline] = useState(typeof navigator === "undefined" ? true : navigator.onLine);
  const engineMode = useSettingsStore((s) => s.settings.engine_mode);
  const onDevice =
    getAppRuntime() === "mobile-tauri" && (engineMode === "local" || engineMode === "standalone");

  useEffect(() => {
    const goOffline = () => setOnline(false);
    const goOnline = () => setOnline(true);
    window.addEventListener("offline", goOffline);
    window.addEventListener("online", goOnline);
    return () => {
      window.removeEventListener("offline", goOffline);
      window.removeEventListener("online", goOnline);
    };
  }, []);

  if (online) return null;
  return (
    <div
      role="status"
      className="mb-3 flex items-center gap-2 rounded-md border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-sm text-sky-300"
    >
      <WifiOff className="h-4 w-4 shrink-0" />
      <span>
        You&apos;re offline.{" "}
        {onDevice
          ? "Scenes stored on this device still play."
          : "Browsing and downloads need a connection."}
      </span>
    </div>
  );
}
