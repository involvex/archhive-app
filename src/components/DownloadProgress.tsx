import { useCallback, useRef, useState } from "react";
import type { DownloadJob } from "@/lib/types";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { Pause, Play, RotateCcw, Trash2, XCircle } from "lucide-react";
import { vibrateTick } from "@/lib/haptics";
import { cn } from "@/lib/utils";

interface DownloadProgressRowProps {
  job: DownloadJob;
  onPause?: (id: string) => void;
  onResume?: (id: string) => void;
  onRetry?: (id: string) => void;
  onCancel?: (id: string) => void;
  onDelete?: (id: string) => void;
}

const statusColors: Record<DownloadJob["status"], string> = {
  pending: "text-yellow-400",
  active: "text-blue-400",
  paused: "text-orange-400",
  waiting_for_wifi: "text-cyan-400",
  completed: "text-green-400",
  failed: "text-red-400",
  cancelled: "text-[var(--color-muted-foreground)]",
};

function getDisplayStatus(job: DownloadJob): { label: string; color: string } {
  if (job.status === "failed" && (job.retry_count ?? 0) > 0) {
    return { label: "retrying", color: "text-orange-400" };
  }
  if (job.status === "waiting_for_wifi") {
    return { label: "waiting for Wi-Fi", color: statusColors.waiting_for_wifi };
  }
  return { label: job.status, color: statusColors[job.status] };
}

function formatRetryInfo(job: DownloadJob): string | null {
  if (job.retry_count == null || job.retry_count === 0) return null;
  const count = job.retry_count;
  const when = job.last_retry_at
    ? new Date(job.last_retry_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : null;
  return when ? `Retry #${count} at ${when}` : `Retry #${count}`;
}

const SWIPE_TRIGGER_PX = 60;
const SWIPE_CLAMP_PX = 120;

export function DownloadProgressRow({
  job,
  onPause,
  onResume,
  onRetry,
  onCancel,
  onDelete,
}: DownloadProgressRowProps) {
  const showProgress =
    job.status === "active" || job.status === "pending" || job.status === "waiting_for_wifi";
  const canRetry =
    job.status === "failed" || job.status === "cancelled" || job.status === "completed";
  const isWaitingForWifi = job.status === "waiting_for_wifi";
  const retryInfo = formatRetryInfo(job);
  const display = getDisplayStatus(job);
  const canPause = job.status === "pending" || job.status === "active";
  const canCancel = job.status === "pending" || job.status === "active" || job.status === "paused";
  const queuePosition =
    job.queue_position != null && (job.status === "pending" || job.status === "waiting_for_wifi")
      ? job.queue_position
      : null;

  // Swipe on touch: right = pause (or resume when paused), left = cancel.
  // Mirrors the SceneCard gesture so the motion is already familiar.
  const [swipeX, setSwipeX] = useState(0);
  const [swiping, setSwiping] = useState(false);
  const startX = useRef(0);
  const startY = useRef(0);
  const isTouch = useRef(false);

  const resetSwipe = useCallback(() => {
    setSwipeX(0);
    setSwiping(false);
  }, []);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    isTouch.current = e.pointerType === "touch";
    startX.current = e.clientX;
    startY.current = e.clientY;
  }, []);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    if (!isTouch.current) return;
    const dx = e.clientX - startX.current;
    const dy = e.clientY - startY.current;
    if (Math.abs(dy) > Math.abs(dx) * 0.6) return;
    if (Math.abs(dx) < 8) return;
    setSwiping(true);
    setSwipeX(Math.max(-SWIPE_CLAMP_PX, Math.min(SWIPE_CLAMP_PX, dx)));
  }, []);

  const onPointerUp = useCallback(() => {
    if (!swiping) {
      resetSwipe();
      return;
    }
    if (Math.abs(swipeX) > SWIPE_TRIGGER_PX) {
      vibrateTick(15);
      if (swipeX < 0 && canCancel) {
        onCancel?.(job.id);
      } else if (swipeX > 0) {
        if (job.status === "paused") onResume?.(job.id);
        else if (canPause) onPause?.(job.id);
      }
    }
    resetSwipe();
  }, [
    swipeX,
    swiping,
    canCancel,
    canPause,
    job.id,
    job.status,
    onCancel,
    onPause,
    onResume,
    resetSwipe,
  ]);

  return (
    <div
      className="relative space-y-2 overflow-hidden rounded-lg border border-[var(--color-border)] p-3"
      style={
        swipeX !== 0
          ? { transform: `translateX(${swipeX}px)`, transition: "transform 0.1s ease" }
          : undefined
      }
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={resetSwipe}
      onPointerLeave={resetSwipe}
    >
      {swipeX !== 0 && (
        <div
          className={cn(
            "pointer-events-none absolute inset-0 flex items-center px-4 transition-opacity",
            Math.abs(swipeX) > 40 ? "opacity-100" : "opacity-40",
            swipeX < 0 ? "justify-end bg-red-500/15" : "justify-start bg-amber-500/15",
          )}
          aria-hidden
        >
          {swipeX < 0 ? (
            <span className="flex items-center gap-1.5 text-xs font-medium text-red-400">
              <XCircle className="h-4 w-4" /> Release to cancel
            </span>
          ) : (
            <span className="flex items-center gap-1.5 text-xs font-medium text-amber-400">
              {job.status === "paused" ? (
                <Play className="h-4 w-4" />
              ) : (
                <Pause className="h-4 w-4" />
              )}
              Release to {job.status === "paused" ? "resume" : "pause"}
            </span>
          )}
        </div>
      )}
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{job.title || job.url}</p>
          <p className={`text-xs capitalize ${display.color}`}>
            {display.label}
            {queuePosition != null && (
              <span className="ml-1.5 rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-[10px] font-semibold normal-case text-[var(--color-muted-foreground)]">
                #{queuePosition} in queue
              </span>
            )}
          </p>
          {retryInfo && (
            <p className="text-[10px] text-[var(--color-muted-foreground)]">{retryInfo}</p>
          )}
        </div>
        <div className="flex shrink-0 gap-1">
          {job.status === "paused" && onResume && (
            <Button variant="ghost" size="icon" title="Resume" onClick={() => onResume(job.id)}>
              <Play className="h-4 w-4" />
            </Button>
          )}
          {isWaitingForWifi && onResume && (
            <Button variant="ghost" size="icon" title="Retry now" onClick={() => onResume(job.id)}>
              <RotateCcw className="h-4 w-4" />
            </Button>
          )}
          {(job.status === "pending" || job.status === "active") && onPause && (
            <Button variant="ghost" size="icon" title="Pause" onClick={() => onPause(job.id)}>
              <Pause className="h-4 w-4" />
            </Button>
          )}
          {canRetry && onRetry && (
            <Button variant="ghost" size="icon" title="Retry" onClick={() => onRetry(job.id)}>
              <RotateCcw className="h-4 w-4" />
            </Button>
          )}
          {(job.status === "pending" || job.status === "active" || job.status === "paused") &&
            onCancel && (
              <Button variant="ghost" size="icon" title="Cancel" onClick={() => onCancel(job.id)}>
                <XCircle className="h-4 w-4" />
              </Button>
            )}
          {onDelete && (
            <Button variant="ghost" size="icon" title="Delete" onClick={() => onDelete(job.id)}>
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>
      {showProgress && <Progress value={job.progress} />}
      {job.error && <p className="text-xs text-red-400">{job.error}</p>}
    </div>
  );
}
