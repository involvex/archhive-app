import { Link, createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { LibraryStats } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Film, FolderOpen, RefreshCw, Tags, Users, GitMerge } from "lucide-react";

export const Route = createFileRoute("/library/")({
  component: LibraryHubPage,
});

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function LibraryHubPage() {
  const [stats, setStats] = useState<LibraryStats | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      const s = await api.getLibraryStats();
      setStats(s);
    } catch {
      setStats(null);
    } finally {
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-2xl font-bold">Library</h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Browse and manage your collection
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={refresh}
          disabled={refreshing}
          className="gap-1.5"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`} />
          Quick scan
        </Button>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium uppercase tracking-wider text-[var(--color-muted-foreground)]">
              Total Scenes
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">
              {stats ? stats.scene_count.toLocaleString() : "—"}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium uppercase tracking-wider text-[var(--color-muted-foreground)]">
              Total Performers
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">
              {stats ? stats.performer_count.toLocaleString() : "—"}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium uppercase tracking-wider text-[var(--color-muted-foreground)]">
              Total Tags
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">
              {stats ? stats.tag_count.toLocaleString() : "—"}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium uppercase tracking-wider text-[var(--color-muted-foreground)]">
              Storage Used
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">
              {stats ? formatBytes(stats.total_size_bytes) : "—"}
            </p>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <Link to="/library/scenes" className="block">
          <Card className="cursor-pointer hover:border-[var(--color-primary)] hover:bg-[var(--color-accent)]/5 transition-colors h-full">
            <CardContent className="flex items-center gap-3 p-4 pt-4">
              <Film className="h-5 w-5 text-[var(--color-primary)] shrink-0" />
              <div>
                <p className="font-semibold">Scenes</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Browse and organize your video library
                </p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link to="/library/performers" className="block">
          <Card className="cursor-pointer hover:border-[var(--color-primary)] hover:bg-[var(--color-accent)]/5 transition-colors h-full">
            <CardContent className="flex items-center gap-3 p-4 pt-4">
              <Users className="h-5 w-5 text-[var(--color-primary)] shrink-0" />
              <div>
                <p className="font-semibold">Performers</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Manage performers and favorites
                </p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link to="/library/tags" className="block">
          <Card className="cursor-pointer hover:border-[var(--color-primary)] hover:bg-[var(--color-accent)]/5 transition-colors h-full">
            <CardContent className="flex items-center gap-3 p-4 pt-4">
              <Tags className="h-5 w-5 text-[var(--color-primary)] shrink-0" />
              <div>
                <p className="font-semibold">Tags</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Explore and organize by tags
                </p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link to="/files" className="block">
          <Card className="cursor-pointer hover:border-[var(--color-primary)] hover:bg-[var(--color-accent)]/5 transition-colors h-full">
            <CardContent className="flex items-center gap-3 p-4 pt-4">
              <FolderOpen className="h-5 w-5 text-[var(--color-primary)] shrink-0" />
              <div>
                <p className="font-semibold">Files</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Browse and manage library files on disk
                </p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link to="/duplicates" className="block">
          <Card className="cursor-pointer hover:border-[var(--color-primary)] hover:bg-[var(--color-accent)]/5 transition-colors h-full">
            <CardContent className="flex items-center gap-3 p-4 pt-4">
              <GitMerge className="h-5 w-5 text-[var(--color-primary)] shrink-0" />
              <div>
                <p className="font-semibold">Duplicates</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  Find and merge duplicate scenes
                </p>
              </div>
            </CardContent>
          </Card>
        </Link>
      </div>
    </div>
  );
}
