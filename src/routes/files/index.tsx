import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { fileStreamUrl, isImageFilePath, isVideoFilePath } from "@/lib/mediaUrl";
import type { FileEntry } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ChevronRight, File, Folder, Film } from "lucide-react";

export const Route = createFileRoute("/files/")({
  component: FilesPage,
});

function formatBytes(bytes?: number): string {
  if (bytes == null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function FilesPage() {
  const [currentPath, setCurrentPath] = useState("");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [playing, setPlaying] = useState<FileEntry | null>(null);

  const refresh = useCallback(() => {
    setLoading(true);
    setError("");
    void api
      .listFiles(currentPath)
      .then((res) => setEntries(res.entries))
      .catch((e) => setError(e instanceof Error ? e.message : "Failed to list files"))
      .finally(() => setLoading(false));
  }, [currentPath]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const crumbs = currentPath ? ["", ...currentPath.split("/").filter(Boolean)] : [""];

  function navigateToCrumb(index: number) {
    if (index === 0) {
      setCurrentPath("");
      setPlaying(null);
      return;
    }
    const parts = currentPath.split("/").filter(Boolean).slice(0, index);
    setCurrentPath(parts.join("/"));
    setPlaying(null);
  }

  function openEntry(entry: FileEntry) {
    if (entry.is_dir) {
      setCurrentPath(entry.path);
      setPlaying(null);
      return;
    }
    if (isVideoFilePath(entry.name)) {
      setPlaying(entry);
    }
  }

  const playUrl = playing ? fileStreamUrl(playing.path) : undefined;

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-2xl font-bold">Files</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Browse your library folder on the desktop host (like a local network file server).
        </p>
      </div>

      <nav className="flex flex-wrap items-center gap-1 text-sm text-[var(--color-muted-foreground)]">
        {crumbs.map((_, index) => {
          const label = index === 0 ? "Library" : crumbs[index];
          return (
            <span key={index} className="flex items-center gap-1">
              {index > 0 && <ChevronRight className="h-3 w-3" />}
              <button
                type="button"
                className="hover:text-[var(--color-foreground)] hover:underline"
                onClick={() => navigateToCrumb(index)}
              >
                {label}
              </button>
            </span>
          );
        })}
      </nav>

      {playUrl && playing && (
        <Card>
          <CardContent className="space-y-2 p-4">
            <p className="text-sm font-medium">{playing.name}</p>
            <video
              key={playUrl}
              src={playUrl}
              controls
              playsInline
              className="aspect-video w-full max-h-[50vh] rounded-md bg-black"
            />
            <Button size="sm" variant="outline" onClick={() => setPlaying(null)}>
              Close player
            </Button>
          </CardContent>
        </Card>
      )}

      {error && <p className="text-sm text-red-500">{error}</p>}
      {loading && <p className="text-sm text-[var(--color-muted-foreground)]">Loading…</p>}

      {!loading && !error && (
        <ul className="divide-y divide-[var(--color-border)] rounded-md border border-[var(--color-border)]">
          {entries.map((entry) => {
            const stream = !entry.is_dir ? fileStreamUrl(entry.path) : undefined;
            return (
              <li key={entry.path}>
                <button
                  type="button"
                  className="flex w-full items-center gap-3 px-3 py-2 text-left text-sm hover:bg-[var(--color-muted)]"
                  onClick={() => openEntry(entry)}
                >
                  {entry.is_dir ? (
                    <Folder className="h-4 w-4 shrink-0 text-[var(--color-primary)]" />
                  ) : isVideoFilePath(entry.name) ? (
                    <Film className="h-4 w-4 shrink-0" />
                  ) : (
                    <File className="h-4 w-4 shrink-0" />
                  )}
                  <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                  {!entry.is_dir && entry.size != null && (
                    <span className="shrink-0 text-xs text-[var(--color-muted-foreground)]">
                      {formatBytes(entry.size)}
                    </span>
                  )}
                </button>
                {stream && isImageFilePath(entry.name) && (
                  <div className="px-3 pb-2">
                    <img
                      src={stream}
                      alt={entry.name}
                      className="max-h-32 rounded object-contain"
                      loading="lazy"
                    />
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      {entries.length === 0 && !loading && !error && (
        <p className="text-sm text-[var(--color-muted-foreground)]">This folder is empty.</p>
      )}

      <p className="text-xs text-[var(--color-muted-foreground)]">
        On your phone browser, open{" "}
        <Link to="/files" className="text-[var(--color-primary)] hover:underline">
          /files
        </Link>{" "}
        on the same LAN host (port 8787).
      </p>
    </div>
  );
}
