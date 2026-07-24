import { useEffect, useState } from "react";
import { sceneMediaUrl, isWebPlayableScene, isHttpMediaSrc, isVideoScene } from "@/lib/mediaUrl";
import { getCapabilities } from "@/lib/runtime";
import { api } from "@/lib/api/client";
import type { Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Pencil, X } from "lucide-react";

interface ScenePlayerDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
  onEdit?: (scene: Scene) => void;
}

function formatBytes(bytes?: number): string | null {
  if (bytes == null) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function ScenePlayerBody({
  scene,
  onClose,
  onEdit,
}: {
  scene: Scene;
  onClose: () => void;
  onEdit?: (scene: Scene) => void;
}) {
  const [detail, setDetail] = useState<Scene | null>(null);
  const caps = getCapabilities();

  useEffect(() => {
    let cancelled = false;
    void api
      .getScene(scene.id)
      .then((data) => {
        if (!cancelled) setDetail(data);
      })
      .catch(console.error);
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

      {webPlayable ? (
        <video
          key={mediaSrc}
          src={mediaSrc}
          controls
          playsInline
          preload="metadata"
          {...(useCors ? { crossOrigin: "anonymous" as const } : {})}
          className="aspect-video w-full rounded-md bg-black"
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

      <dl className="mt-4 space-y-2 text-sm">
        {data.performers.length > 0 && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
            <dd>{data.performers.join(", ")}</dd>
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

export function ScenePlayerDialog({ scene, open, onClose, onEdit }: ScenePlayerDialogProps) {
  if (!open || !scene) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center bg-black/80 p-0 sm:items-center sm:p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="flex max-h-[92dvh] w-full max-w-4xl flex-col overflow-hidden rounded-t-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-xl sm:rounded-lg"
      >
        <div className="overflow-y-auto overscroll-contain p-4">
          <ScenePlayerBody key={scene.id} scene={scene} onClose={onClose} onEdit={onEdit} />
        </div>
      </div>
    </div>
  );
}
