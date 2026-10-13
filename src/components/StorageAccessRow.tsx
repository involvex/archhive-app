import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getAppRuntime } from "@/lib/runtime";
import { Button } from "@/components/ui/button";

/**
 * Android "All files access": lets ArcHive delete library files in shared
 * storage (e.g. Download/ArcHive) that this install didn't create, such as
 * downloads from before an uninstall. Phone app only.
 */
export function StorageAccessRow() {
  const [granted, setGranted] = useState<boolean | null>(null);
  const mobile = getAppRuntime() === "mobile-tauri";

  useEffect(() => {
    if (!mobile) return;
    const check = () => void api.storageAccess().then(setGranted);
    check();
    // Re-check when returning from the system settings screen.
    const onVisible = () => {
      if (document.visibilityState === "visible") check();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [mobile]);

  if (!mobile || granted === null) return null;
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-[var(--color-border)] p-2.5">
      <div className="min-w-0">
        <p className="text-sm font-medium">All files access</p>
        <p className="text-xs text-[var(--color-muted-foreground)]">
          {granted
            ? "Allowed — ArcHive can delete any file in its library folder."
            : "Needed to delete library files in shared folders (e.g. Download/ArcHive) that ArcHive didn't create itself."}
        </p>
      </div>
      {!granted && (
        <Button
          size="sm"
          variant="outline"
          className="shrink-0"
          onClick={() => void api.openStorageAccessSettings().catch(() => {})}
        >
          Allow
        </Button>
      )}
    </div>
  );
}
