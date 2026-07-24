import { sceneMediaUrl, isWebPlayableScene, isHttpMediaSrc, isVideoScene } from "@/lib/mediaUrl";
import { getCapabilities } from "@/lib/runtime";
import { api } from "@/lib/api/client";
import type { Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { X } from "lucide-react";

interface ScenePlayerDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
}

export function ScenePlayerDialog({ scene, open, onClose }: ScenePlayerDialogProps) {
  if (!open || !scene) return null;

  const caps = getCapabilities();
  const mediaSrc = sceneMediaUrl(scene);
  const webPlayable = isWebPlayableScene(scene) && mediaSrc;
  const useCors = isHttpMediaSrc(mediaSrc);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="w-full max-w-4xl rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <div className="mb-3 flex items-center justify-between gap-2">
          <h3 className="text-lg font-semibold line-clamp-2">{scene.title}</h3>
          <button
            type="button"
            onClick={onClose}
            className="shrink-0 rounded p-1 hover:bg-[var(--color-muted)]"
          >
            <X className="h-4 w-4" />
          </button>
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
          />
        ) : (
          <div className="space-y-3">
            <p className="text-sm text-[var(--color-muted-foreground)]">
              {scene.path
                ? isVideoScene(scene)
                  ? "This container (e.g. MKV/AVI) often cannot play in the in-app player. Open it with your system player instead."
                  : "This file format may not play in the browser."
                : "No media file path for this scene."}
            </p>
            {caps.localIpc && scene.path && (
              <Button
                variant="default"
                onClick={() => void api.openSceneWithDefault(scene.id).catch(console.error)}
              >
                Open with system player
              </Button>
            )}
          </div>
        )}
        <div className="mt-4 flex justify-end">
          <Button variant="outline" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
