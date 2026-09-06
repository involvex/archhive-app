import { useCallback, useEffect, useRef, useState } from "react";
import { sceneMediaUrl, isWebPlayableScene, isHttpMediaSrc, isVideoScene } from "@/lib/mediaUrl";
import { getCapabilities } from "@/lib/runtime";
import { useRecentlyViewedStore } from "@/lib/stores/recentlyViewed";
import { api } from "@/lib/api/client";
import type { Scene, WatchProgress } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { formatDuration } from "@/components/SceneCard";
import { ChevronLeft, ChevronRight, Pencil, Play, RotateCcw, X } from "lucide-react";

interface ScenePlayerDialogProps {
  scene: Scene | null;
  scenes?: Scene[];
  currentIndex?: number;
  open: boolean;
  onClose: () => void;
  onEdit?: (scene: Scene) => void;
  onNavigate?: (scene: Scene, index: number) => void;
}

function formatBytes(bytes?: number): string | null {
  if (bytes == null) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function ScenePlayerBody({
  scene,
  scenes,
  currentIndex,
  onClose,
  onEdit,
  onNavigate,
}: {
  scene: Scene;
  scenes?: Scene[];
  currentIndex?: number;
  onClose: () => void;
  onEdit?: (scene: Scene) => void;
  onNavigate?: (scene: Scene, index: number) => void;
}) {
  const [detail, setDetail] = useState<Scene | null>(null);
  const caps = getCapabilities();
  // #26 watch-history tracking.
  const videoRef = useRef<HTMLVideoElement>(null);
  const posRef = useRef(0);
  const durRef = useRef(0);
  const lastSavedRef = useRef(0);
  const [resume, setResume] = useState<WatchProgress | null>(null);

  useEffect(() => {
    let cancelled = false;
    void api
      .getScene(scene.id)
      .then((data) => {
        if (!cancelled) setDetail(data);
      })
      .catch(console.error);
    // Offer resume when a meaningful position was stored.
    void api
      .getWatchProgress(scene.id)
      .then((p) => {
        if (cancelled || !p) return;
        const resumable =
          p.position_secs > 10 &&
          (p.duration_secs <= 0 || p.position_secs < p.duration_secs * 0.95);
        if (resumable) setResume(p);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [scene.id]);

  const data = detail ?? scene;
  const mediaSrc = sceneMediaUrl(data);
  const webPlayable = isWebPlayableScene(data) && mediaSrc;
  // Avoid crossOrigin on Android WebView when possible — it can blank playback on CORS hiccups.
  const useCors = isHttpMediaSrc(mediaSrc) && caps.localIpc;
  const fileSize = formatBytes(data.file_size);

  const persist = useCallback(
    (position: number, duration: number) => {
      posRef.current = position;
      durRef.current = duration;
      lastSavedRef.current = Date.now();
      void api.recordWatchProgress(scene.id, position, duration).catch(() => {});
    },
    [scene.id],
  );

  function handleTimeUpdate() {
    const el = videoRef.current;
    if (!el || !Number.isFinite(el.currentTime)) return;
    const now = Date.now();
    if (now - lastSavedRef.current > 5000 && Math.abs(el.currentTime - posRef.current) > 1) {
      persist(el.currentTime, Number.isFinite(el.duration) ? el.duration : 0);
    } else {
      posRef.current = el.currentTime;
      if (Number.isFinite(el.duration)) durRef.current = el.duration;
    }
  }

  function handlePause() {
    const el = videoRef.current;
    if (!el) return;
    persist(el.currentTime, Number.isFinite(el.duration) ? el.duration : durRef.current);
  }

  // Flush final position when the dialog closes / scene changes.
  useEffect(() => {
    return () => {
      if (posRef.current > 0) {
        void api.recordWatchProgress(scene.id, posRef.current, durRef.current).catch(() => {});
      }
    };
  }, [scene.id]);

  function resumePlayback() {
    const el = videoRef.current;
    if (el && resume) {
      try {
        el.currentTime = resume.position_secs;
      } catch {
        /* seek before metadata — timeupdate will catch up */
      }
      void el.play().catch(() => {});
    }
    posRef.current = resume?.position_secs ?? 0;
    setResume(null);
  }

  return (
    <>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h3 className="text-lg font-semibold leading-snug line-clamp-2">{data.title}</h3>
        <div className="flex shrink-0 items-center gap-1">
          {onEdit && (
            <button
              type="button"
              onClick={() => onEdit(data)}
              className="rounded p-2 hover:bg-[var(--color-muted)]"
              aria-label="Edit scene"
            >
              <Pencil className="h-4 w-4" />
            </button>
          )}
          <button
            type="button"
            onClick={onClose}
            className="rounded p-2 hover:bg-[var(--color-muted)]"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>

      {resume && (
        <div className="mb-3 flex flex-wrap items-center gap-2 rounded-md border border-[var(--color-primary)]/40 bg-[var(--color-primary)]/10 px-3 py-2">
          <p className="text-sm">Resume from {formatDuration(Math.floor(resume.position_secs))}?</p>
          <div className="flex-1" />
          <Button size="sm" onClick={resumePlayback}>
            <Play className="h-3.5 w-3.5" />
            Resume
          </Button>
          <Button size="sm" variant="outline" onClick={() => setResume(null)}>
            <RotateCcw className="h-3.5 w-3.5" />
            Start over
          </Button>
        </div>
      )}

      {webPlayable ? (
        <video
          key={mediaSrc}
          ref={videoRef}
          src={mediaSrc}
          controls
          playsInline
          preload="metadata"
          {...(useCors ? { crossOrigin: "anonymous" as const } : {})}
          className="aspect-video w-full rounded-md bg-black"
          onTimeUpdate={handleTimeUpdate}
          onPause={handlePause}
          onError={() => console.error("Video playback failed", mediaSrc)}
        />
      ) : (
        <div className="space-y-3 rounded-md border border-[var(--color-border)] bg-[var(--color-muted)] p-3">
          <p className="text-sm text-[var(--color-muted-foreground)]">
            {data.path
              ? isVideoScene(data)
                ? "This container (e.g. MKV/AVI) often cannot play in the in-app player."
                : "This file format may not play in the browser."
              : "No media file path for this scene."}
          </p>
          {caps.localIpc && data.path && (
            <Button
              variant="default"
              onClick={() => void api.openSceneWithDefault(data.id).catch(console.error)}
            >
              Open with system player
            </Button>
          )}
        </div>
      )}

      {scenes && scenes.length > 1 && currentIndex != null && onNavigate && (
        <div className="mt-3 flex items-center justify-between gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-muted)] px-3 py-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={currentIndex <= 0}
            onClick={() => onNavigate(scenes[currentIndex - 1], currentIndex - 1)}
            className="min-h-10"
          >
            <ChevronLeft className="h-4 w-4" />
            Previous
          </Button>
          <span className="text-xs text-[var(--color-muted-foreground)]">
            {currentIndex + 1} / {scenes.length}
          </span>
          <Button
            variant="ghost"
            size="sm"
            disabled={currentIndex >= scenes.length - 1}
            onClick={() => onNavigate(scenes[currentIndex + 1], currentIndex + 1)}
            className="min-h-10"
          >
            Next
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      )}

      <dl className="mt-4 space-y-2 text-sm">
        {data.performers.length > 0 && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
            <dd>{data.performers.join(", ")}</dd>
          </div>
        )}
        {data.channel && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Channel</dt>
            <dd>{data.channel}</dd>
          </div>
        )}
        {data.tags.length > 0 && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Tags</dt>
            <dd className="mt-1 flex flex-wrap gap-1">
              {data.tags.map((tag) => (
                <span
                  key={tag}
                  className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-xs"
                >
                  {tag}
                </span>
              ))}
            </dd>
          </div>
        )}
        {data.studio_name && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Studio</dt>
            <dd>{data.studio_name}</dd>
          </div>
        )}
        {data.date && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Date</dt>
            <dd>{data.date}</dd>
          </div>
        )}
        {fileSize && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">File size</dt>
            <dd>{fileSize}</dd>
          </div>
        )}
        {data.width != null && data.height != null && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Resolution</dt>
            <dd>
              {data.width}×{data.height}
            </dd>
          </div>
        )}
        {data.source_url && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Source</dt>
            <dd className="break-all">
              <a
                href={data.source_url}
                target="_blank"
                rel="noreferrer"
                className="text-[var(--color-primary)] hover:underline"
              >
                {data.source_url}
              </a>
            </dd>
          </div>
        )}
        {data.path && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Path</dt>
            <dd className="break-all font-mono text-xs text-[var(--color-muted-foreground)]">
              {data.path}
            </dd>
          </div>
        )}
      </dl>

      <div className="mt-4 flex flex-wrap justify-end gap-2">
        {onEdit && (
          <Button
            variant="outline"
            onClick={() => onEdit(data)}
            className="min-h-10 min-w-[5.5rem]"
          >
            <Pencil className="h-3.5 w-3.5" />
            Edit
          </Button>
        )}
        <Button variant="outline" onClick={onClose} className="min-h-10 min-w-[5.5rem]">
          Close
        </Button>
      </div>
    </>
  );
}

export function ScenePlayerDialog({
  scene,
  scenes,
  currentIndex,
  open,
  onClose,
  onEdit,
  onNavigate,
}: ScenePlayerDialogProps) {
  // Q11: record every opened scene for the "Continue watching" rail.
  useEffect(() => {
    if (open && scene) {
      useRecentlyViewedStore.getState().record(scene);
    }
  }, [open, scene]);

  useEffect(() => {
    if (!open || !scenes || currentIndex == null) return;
    const idx = currentIndex;
    const list = scenes;
    if (!onNavigate) return;
    const nav = onNavigate;
    function handleKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        if (idx > 0) {
          nav(list[idx - 1], idx - 1);
        }
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        if (idx < list.length - 1) {
          nav(list[idx + 1], idx + 1);
        }
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, scenes, currentIndex, onNavigate]);

  if (!open || !scene) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center bg-black/80 p-0 sm:items-center sm:p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="flex max-h-[92dvh] w-full max-w-4xl flex-col overflow-hidden rounded-t-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-xl sm:rounded-lg"
      >
        <div className="overflow-y-auto overscroll-contain p-4">
          <ScenePlayerBody
            key={scene.id}
            scene={scene}
            scenes={scenes}
            currentIndex={currentIndex}
            onClose={onClose}
            onEdit={onEdit}
            onNavigate={onNavigate}
          />
        </div>
      </div>
    </div>
  );
}
