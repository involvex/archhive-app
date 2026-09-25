import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "react-hot-toast";
import { Pause, Play, RotateCcw, Trash2, HardDrive, BatteryCharging, X } from "lucide-react";
import { api } from "@/lib/api/client";
import type { DownloadJob } from "@/lib/types";
import { isDesktopTauri, isMobileDevice } from "@/lib/tauri";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { BulkImportPanel } from "@/components/BulkImportPanel";
import { DownloadGuardDialog } from "@/components/DownloadGuardDialog";
import { Button } from "@/components/ui/button";
import { useDownloadNotifications } from "@/lib/hooks/useDownloadNotifications";
import { useSettingsStore } from "@/lib/stores/settings";
import {
  checkDownloadGuards,
  formatBytes,
  STORAGE_WARN_BYTES,
  type GuardBlock,
} from "@/lib/downloads/guards";

export const Route = createFileRoute("/downloads/")({
  component: DownloadsPage,
});

function DownloadsPage() {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);
  const [freeBytes, setFreeBytes] = useState<number | null>(null);
  const [guard, setGuard] = useState<GuardBlock>(null);
  const [pendingAction, setPendingAction] = useState<(() => void) | null>(null);
  const wifiOnly = useSettingsStore((s) => s.settings.download_on_wifi_only ?? true);

  // In-app complete/failed toasts (desktop also gets system notifications;
  // mobile relies on these plus the foreground-service channel).
  useDownloadNotifications(toast);

  const refreshJobs = useCallback(() => {
    void api.listDownloads().then(setJobs).catch(console.error);
  }, []);

  const refreshStorage = useCallback(() => {
    void api
      .getLibraryStats()
      .then((s) => setFreeBytes(s.free_space_bytes))
      .catch(() => setFreeBytes(null));
  }, []);

  const failedIds = useMemo(
    () => jobs.filter((j) => j.status === "failed" || j.status === "cancelled").map((j) => j.id),
    [jobs],
  );

  const completedIds = useMemo(
    () =>
      jobs
        .filter(
          (j) => j.status === "completed" || j.status === "failed" || j.status === "cancelled",
        )
        .map((j) => j.id),
    [jobs],
  );

  const pausableIds = useMemo(
    () => jobs.filter((j) => j.status === "pending" || j.status === "active").map((j) => j.id),
    [jobs],
  );

  const resumableIds = useMemo(
    () =>
      jobs.filter((j) => j.status === "paused" || j.status === "waiting_for_wifi").map((j) => j.id),
    [jobs],
  );

  const pendingCount = useMemo(
    () => jobs.filter((j) => j.status === "pending" || j.status === "waiting_for_wifi").length,
    [jobs],
  );

  /** Run a bulk action behind the metered/storage guards. */
  const guarded = useCallback(
    (actionLabel: string, run: () => void) => {
      const block = checkDownloadGuards({ wifiOnly, freeBytes });
      if (!block) {
        run();
        return;
      }
      setPendingAction(() => run);
      setGuard(block);
      void actionLabel;
    },
    [wifiOnly, freeBytes],
  );

  const runBulk = useCallback(
    async (ids: string[], fn: (id: string) => Promise<void>) => {
      for (const id of ids) {
        try {
          await fn(id);
        } catch (e) {
          console.error("bulk action failed", id, e);
        }
      }
      refreshJobs();
    },
    [refreshJobs],
  );

  const retryFailed = useCallback(async () => {
    await runBulk(failedIds, (id) => api.retryDownload(id));
  }, [failedIds, runBulk]);

  const pauseAll = useCallback(async () => {
    await runBulk(pausableIds, (id) => api.pauseDownload(id));
    toast.success(`Paused ${pausableIds.length} download${pausableIds.length === 1 ? "" : "s"}`);
  }, [pausableIds, runBulk]);

  const resumeAll = useCallback(async () => {
    guarded("Resume anyway", () => {
      void runBulk(resumableIds, (id) => api.resumeDownload(id)).then(() =>
        toast.success(
          `Resumed ${resumableIds.length} download${resumableIds.length === 1 ? "" : "s"}`,
        ),
      );
    });
  }, [guarded, resumableIds, runBulk]);

  const clearCompleted = useCallback(async () => {
    if (
      !window.confirm(
        `Remove ${completedIds.length} completed/failed/cancelled download(s) from the queue?\nThis does not delete the downloaded files.`,
      )
    ) {
      return;
    }
    for (const id of completedIds) {
      try {
        await api.deleteDownload(id);
      } catch (e) {
        console.error("delete failed", id, e);
      }
    }
    refreshJobs();
  }, [completedIds, refreshJobs]);

  useEffect(() => {
    refreshJobs();
    refreshStorage();
    const unsubs: Array<() => void> = [];
    void api
      .subscribeDownloadProgress((job) => {
        setJobs((prev) => {
          const idx = prev.findIndex((j) => j.id === job.id);
          if (idx === -1) return [job, ...prev];
          const next = [...prev];
          next[idx] = job;
          return next;
        });
      })
      .then((fn) => unsubs.push(fn));
    if (isDesktopTauri()) {
      void listen<string>("download:deleted", (e) => {
        const id = e.payload;
        setJobs((prev) => prev.filter((j) => j.id !== id));
      }).then((fn) => unsubs.push(fn));
    }
    const storageTimer = window.setInterval(refreshStorage, 30000);
    return () => {
      for (const fn of unsubs) fn();
      window.clearInterval(storageTimer);
    };
  }, [refreshJobs, refreshStorage]);

  const lowSpace = freeBytes != null && freeBytes < STORAGE_WARN_BYTES;

  // Battery-optimization guidance (mobile only): Android kills background
  // downloads unless the app is exempted. Dismissible, remembered per device.
  const [battDismissed, setBattDismissed] = useState(
    () =>
      typeof localStorage !== "undefined" &&
      localStorage.getItem("archhive:batt-opt-dismissed") === "1",
  );
  const hasBackgroundWork = useMemo(
    () =>
      jobs.some(
        (j) => j.status === "active" || j.status === "pending" || j.status === "waiting_for_wifi",
      ),
    [jobs],
  );
  const showBattBanner = isMobileDevice() && !battDismissed && hasBackgroundWork;
  const dismissBattBanner = useCallback(() => {
    try {
      localStorage.setItem("archhive:batt-opt-dismissed", "1");
    } catch {
      /* private mode — banner just reappears next visit */
    }
    setBattDismissed(true);
  }, []);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-bold">
          Downloads
          {pendingCount > 0 && (
            <span className="ml-2 align-middle text-sm font-medium text-[var(--color-muted-foreground)]">
              {pendingCount} queued
            </span>
          )}
        </h2>
        <div className="flex flex-wrap gap-2">
          {pausableIds.length > 0 && (
            <Button variant="outline" size="sm" onClick={() => void pauseAll()}>
              <Pause className="mr-1.5 h-4 w-4" />
              Pause all ({pausableIds.length})
            </Button>
          )}
          {resumableIds.length > 0 && (
            <Button variant="outline" size="sm" onClick={() => void resumeAll()}>
              <Play className="mr-1.5 h-4 w-4" />
              Resume all ({resumableIds.length})
            </Button>
          )}
          {failedIds.length > 0 && (
            <Button variant="outline" size="sm" onClick={() => void retryFailed()}>
              <RotateCcw className="mr-1.5 h-4 w-4" />
              Retry failed ({failedIds.length})
            </Button>
          )}
          {completedIds.length > 0 && (
            <Button variant="outline" size="sm" onClick={() => void clearCompleted()}>
              <Trash2 className="mr-1.5 h-4 w-4" />
              Clear completed ({completedIds.length})
            </Button>
          )}
        </div>
      </div>
      {showBattBanner && (
        <div className="flex items-start gap-2 rounded-md border border-emerald-400/30 bg-emerald-400/10 px-3 py-2 text-sm text-emerald-300">
          <BatteryCharging className="mt-0.5 h-4 w-4 shrink-0" />
          <span className="min-w-0 flex-1">
            Keep background downloads alive: Android Settings → Apps → ArcHive → Battery →
            Unrestricted. Otherwise the OS may pause downloads when the screen is off.
          </span>
          <button
            type="button"
            onClick={dismissBattBanner}
            aria-label="Dismiss battery tip"
            className="rounded p-1.5 hover:bg-white/10"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}
      {lowSpace && freeBytes != null && (
        <div className="flex items-center gap-2 rounded-md border border-amber-400/30 bg-amber-400/10 px-3 py-2 text-sm text-amber-400">
          <HardDrive className="h-4 w-4 shrink-0" />
          <span>
            Low storage — {formatBytes(freeBytes)} free. New downloads may fail; clear completed
            items or free up device space.
          </span>
        </div>
      )}
      <BulkImportPanel onQueued={refreshJobs} />
      <div className="space-y-2">
        {jobs.map((job) => (
          <DownloadProgressRow
            key={job.id}
            job={job}
            onPause={(id) => void api.pauseDownload(id).then(refreshJobs)}
            onResume={(id) => void api.resumeDownload(id).then(refreshJobs)}
            onRetry={(id) => void api.retryDownload(id).then(refreshJobs)}
            onCancel={(id) => void api.cancelDownload(id).then(refreshJobs)}
            onDelete={(id) => void api.deleteDownload(id).then(refreshJobs)}
          />
        ))}
        {jobs.length === 0 && (
          <p className="text-sm text-[var(--color-muted-foreground)]">No downloads yet.</p>
        )}
      </div>
      {guard && (
        <DownloadGuardDialog
          block={guard}
          actionLabel={guard.kind === "cellular" ? "Download anyway" : "Continue anyway"}
          onConfirm={() => {
            const run = pendingAction;
            setGuard(null);
            setPendingAction(null);
            run?.();
          }}
          onCancel={() => {
            setGuard(null);
            setPendingAction(null);
          }}
        />
      )}
    </div>
  );
}
