import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { DirBrowseResponse } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { ArrowUp, Folder, Home, Loader2 } from "lucide-react";

interface DirectoryPickerDialogProps {
  open: boolean;
  initialPath: string;
  onSelect: (path: string) => void;
  onClose: () => void;
}

/**
 * In-app folder browser for picking the download output directory.
 * Used on mobile, where the native dialog plugin can only pick files
 * (ACTION_GET_CONTENT) — not directories.
 */
export function DirectoryPickerDialog({
  open,
  initialPath,
  onSelect,
  onClose,
}: DirectoryPickerDialogProps) {
  const [data, setData] = useState<DirBrowseResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const load = useCallback(async (path?: string) => {
    setLoading(true);
    setError("");
    try {
      setData(await api.browseDirs(path));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Cannot list folder");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch-on-open pattern
    if (open) void load(initialPath || undefined);
  }, [open, initialPath, load]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Choose download folder"
        className="flex max-h-[90vh] w-full max-w-lg flex-col rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <p className="text-sm font-medium">Choose download folder</p>
        <p className="mt-1 truncate font-mono text-xs text-[var(--color-muted-foreground)]">
          {data?.current ?? initialPath}
        </p>

        <div className="mt-2 flex gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={!data?.parent || loading}
            onClick={() => data?.parent && void load(data.parent)}
            className="min-h-10"
          >
            <ArrowUp className="mr-1 h-4 w-4" /> Up
          </Button>
          {data?.roots.map((root) => (
            <Button
              key={root.path}
              variant="outline"
              size="sm"
              disabled={loading}
              onClick={() => void load(root.path)}
              className="min-h-10"
            >
              <Home className="mr-1 h-4 w-4" />
              {root.name}
            </Button>
          ))}
        </div>

        <div className="mt-2 min-h-40 flex-1 overflow-y-auto rounded-md border border-[var(--color-border)]">
          {loading && (
            <p className="flex items-center gap-2 p-4 text-sm text-[var(--color-muted-foreground)]">
              <Loader2 className="h-4 w-4 animate-spin" /> Loading…
            </p>
          )}
          {!loading && error && <p className="p-4 text-sm text-red-400">{error}</p>}
          {!loading && !error && data?.dirs.length === 0 && (
            <p className="p-4 text-sm text-[var(--color-muted-foreground)]">No subfolders.</p>
          )}
          {!loading &&
            !error &&
            data?.dirs.map((dir) => (
              <button
                key={dir.path}
                type="button"
                onClick={() => void load(dir.path)}
                className="flex w-full items-center gap-2 px-3 py-2.5 text-left text-sm hover:bg-[var(--color-muted)] active:bg-[var(--color-muted)]"
              >
                <Folder className="h-4 w-4 shrink-0 text-[var(--color-primary)]" />
                <span className="truncate">{dir.name}</span>
              </button>
            ))}
        </div>

        {data && !data.can_write && !loading && (
          <p className="mt-2 text-xs text-yellow-400">
            This folder is not writable by the app — downloads would fail here.
          </p>
        )}

        <div className="mt-3 flex justify-end gap-2">
          <Button variant="outline" onClick={onClose} className="min-h-10 min-w-[5.5rem]">
            Cancel
          </Button>
          <Button
            variant="default"
            disabled={!data || !data.can_write}
            title={
              data && !data.can_write ? "Folder is not writable by the app" : "Use this folder"
            }
            onClick={() => {
              if (data) onSelect(data.current);
            }}
            className="min-h-10 min-w-[5.5rem]"
          >
            Use this folder
          </Button>
        </div>
      </div>
    </div>
  );
}
