import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { sceneThumbUrl } from "@/lib/mediaUrl";
import type { Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { X } from "lucide-react";

interface SceneDetailsDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
}

function formatBytes(bytes?: number): string | null {
  if (bytes == null) return null;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function SceneDetailsBody({ scene, onClose }: { scene: Scene; onClose: () => void }) {
  const [detail, setDetail] = useState<Scene | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void api
      .getScene(scene.id)
      .then((data) => {
        if (!cancelled) setDetail(data);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load details");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [scene.id]);

  const data = detail ?? scene;
  const thumbSrc = sceneThumbUrl(data);
  const fileSize = formatBytes(data.file_size);

  return (
    <>
      <div className="mb-4 flex items-center justify-between gap-2">
        <h3 className="text-lg font-semibold line-clamp-2">{data.title}</h3>
        <button
          type="button"
          onClick={onClose}
          className="shrink-0 rounded p-1 hover:bg-[var(--color-muted)]"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {thumbSrc && (
        <div className="mb-4 aspect-video overflow-hidden rounded-md bg-[var(--color-muted)]">
          <img src={thumbSrc} alt={data.title} className="h-full w-full object-cover" />
        </div>
      )}

      {loading && <p className="text-sm text-[var(--color-muted-foreground)]">Loading details…</p>}
      {error && <p className="text-sm text-red-500">{error}</p>}

      <dl className="space-y-2 text-sm">
        {data.path && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">Path</dt>
            <dd className="break-all font-mono text-xs">{data.path}</dd>
          </div>
        )}
        {data.source_url && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">Source URL</dt>
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
        {fileSize && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">File size</dt>
            <dd>{fileSize}</dd>
          </div>
        )}
        {data.phash && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">pHash</dt>
            <dd className="break-all font-mono text-xs">{data.phash}</dd>
          </div>
        )}
        {data.oshash && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">oShash</dt>
            <dd className="break-all font-mono text-xs">{data.oshash}</dd>
          </div>
        )}
        {data.performers.length > 0 && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">Performers</dt>
            <dd>{data.performers.join(", ")}</dd>
          </div>
        )}
        {data.tags.length > 0 && (
          <div>
            <dt className="text-[var(--color-muted-foreground)]">Tags</dt>
            <dd className="flex flex-wrap gap-1">
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
      </dl>

      <div className="mt-4 flex justify-end">
        <Button variant="outline" onClick={onClose}>
          Close
        </Button>
      </div>
    </>
  );
}

export function SceneDetailsDialog({ scene, open, onClose }: SceneDetailsDialogProps) {
  if (!open || !scene) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="w-full max-w-lg rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <SceneDetailsBody key={scene.id} scene={scene} onClose={onClose} />
      </div>
    </div>
  );
}
