import type { Scene } from "@/lib/types";

/** Prefer higher resolution, then larger file size. */
export function pickBestDuplicateScene(scenes: Scene[]): Scene | null {
  if (scenes.length === 0) return null;
  return [...scenes].sort((a, b) => {
    const aPixels = (a.width ?? 0) * (a.height ?? 0);
    const bPixels = (b.width ?? 0) * (b.height ?? 0);
    if (bPixels !== aPixels) return bPixels - aPixels;
    const aSize = a.file_size ?? 0;
    const bSize = b.file_size ?? 0;
    if (bSize !== aSize) return bSize - aSize;
    return (b.duration ?? 0) - (a.duration ?? 0);
  })[0];
}

export function formatFileSize(bytes?: number): string {
  if (bytes == null || bytes <= 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function formatDuration(seconds?: number): string {
  if (seconds == null || seconds <= 0) return "—";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function formatResolution(width?: number, height?: number): string {
  if (!width || !height) return "—";
  return `${width}×${height}`;
}
