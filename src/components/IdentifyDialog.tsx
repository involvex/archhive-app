import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene, StashBoxEndpoint, StashSceneMatch } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Check, Play, X } from "lucide-react";

interface IdentifyDialogProps {
  open: boolean;
  onClose: () => void;
  /** Parent refreshes its scene list + banner count after batch work. */
  onDone: () => void;
}

type RowState = "pending" | "working" | "matched" | "nomatch" | "error" | "applying" | "applied";

interface IdentifyRow {
  scene: Scene;
  state: RowState;
  match: StashSceneMatch | null;
  error?: string;
}

const BATCH_LIMIT = 25;

/**
 * Batch Identify (Stash Tagger-style): fingerprint-only matching for every
 * unenriched scene. No title fallback in batch mode — a wrong auto-match is
 * worse than no match. Each row is applied explicitly (single or Apply all).
 */
export function IdentifyDialog({ open, onClose, onDone }: IdentifyDialogProps) {
  const [endpoints, setEndpoints] = useState<StashBoxEndpoint[]>([]);
  const [endpointId, setEndpointId] = useState("");
  const [rows, setRows] = useState<IdentifyRow[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [running, setRunning] = useState(false);
  const [applyingAll, setApplyingAll] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cancelRef = useRef(false);
  const dirtyRef = useRef(false);

  useEffect(() => {
    if (!open) return;
    dirtyRef.current = false;
    cancelRef.current = false;
    setRows([]);
    setTotal(0);
    setError(null);
    setLoading(true);
    let cancelled = false;
    void (async () => {
      try {
        const eps = await api.listStashboxEndpoints();
        if (cancelled) return;
        setEndpoints(eps);
        setEndpointId(eps[0]?.id ?? "");
        if (eps.length === 0) {
          setError("No stash-box endpoint configured — add one in Settings → Metadata.");
          return;
        }
        const result = await api.listUnenrichedScenes(BATCH_LIMIT);
        if (cancelled) return;
        setTotal(result.total);
        setRows(
          result.scenes.map((scene) => ({ scene, state: "pending" as RowState, match: null })),
        );
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load scenes");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open]);

  function close() {
    if (dirtyRef.current) onDone();
    onClose();
  }

  /** Capped-concurrency pool: stash-box RTT dominates, so a few in flight
   *  beats sequential without hammering the box. Rows update independently
   *  via functional setState, so order doesn't matter. */
  async function runPooled<T>(items: T[], size: number, fn: (item: T) => Promise<void>) {
    let next = 0;
    const workers = Array.from({ length: Math.max(1, Math.min(size, items.length)) }, () =>
      (async () => {
        while (next < items.length) {
          if (cancelRef.current) return;
          const item = items[next];
          next += 1;
          await fn(item);
        }
      })(),
    );
    await Promise.all(workers);
  }

  async function runBatch() {
    if (running || rows.length === 0 || !endpointId) return;
    setRunning(true);
    setError(null);
    cancelRef.current = false;
    const endpoint = endpointId;
    const targets = rows.filter((r) => r.state === "pending" || r.state === "error");
    await runPooled(targets, 5, async (row) => {
      setRows((prev) =>
        prev.map((r) => (r.scene.id === row.scene.id ? { ...r, state: "working" } : r)),
      );
      try {
        const matches = await api.queryStashdbForScene(row.scene.id, endpoint, true);
        const best = matches[0] ?? null;
        setRows((prev) =>
          prev.map((r) =>
            r.scene.id === row.scene.id
              ? { ...r, state: best ? "matched" : "nomatch", match: best }
              : r,
          ),
        );
      } catch (e) {
        setRows((prev) =>
          prev.map((r) =>
            r.scene.id === row.scene.id
              ? {
                  ...r,
                  state: "error",
                  error: e instanceof Error ? e.message : "Query failed",
                }
              : r,
          ),
        );
      }
    });
    setRunning(false);
  }

  async function applyRow(row: IdentifyRow) {
    if (!row.match || row.state === "applying" || row.state === "applied") return;
    setRows((prev) =>
      prev.map((r) => (r.scene.id === row.scene.id ? { ...r, state: "applying" } : r)),
    );
    try {
      await api.applyStashdbMatch({
        scene_id: row.scene.id,
        endpoint_id: endpointId,
        stash_id: row.match.stash_id,
        apply_title: true,
        apply_date: true,
        apply_studio: true,
        apply_performers: true,
        apply_tags: true,
        apply_image_as_thumb: false,
      });
      dirtyRef.current = true;
      setRows((prev) =>
        prev.map((r) => (r.scene.id === row.scene.id ? { ...r, state: "applied" } : r)),
      );
    } catch (e) {
      setRows((prev) =>
        prev.map((r) =>
          r.scene.id === row.scene.id
            ? { ...r, state: "error", error: e instanceof Error ? e.message : "Apply failed" }
            : r,
        ),
      );
    }
  }

  async function applyAll() {
    if (applyingAll) return;
    const targets = rows.filter((r) => r.state === "matched");
    if (targets.length === 0) return;
    setApplyingAll(true);
    cancelRef.current = false;
    // Applies are writes — keep parallelism low.
    await runPooled(targets, 3, (row) => applyRow(row));
    setApplyingAll(false);
  }

  if (!open) return null;
  const matchedCount = rows.filter((r) => r.state === "matched").length;
  const appliedCount = rows.filter((r) => r.state === "applied").length;
  const doneCount = rows.filter((r) => !["pending", "working"].includes(r.state)).length;

  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Identify scenes from StashDB"
        className="max-h-[92dvh] w-full max-w-lg overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <div className="mb-3 flex items-center justify-between gap-2">
          <h3 className="text-lg font-semibold">Identify from StashDB</h3>
          <button
            type="button"
            onClick={close}
            className="shrink-0 rounded p-1 hover:bg-[var(--color-muted)]"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <p className="mb-3 text-xs text-[var(--color-muted-foreground)]">
          Fingerprint-only matching (no title guessing). Apply per scene or all at once — title,
          date, studio, performers and tags merge in.
        </p>

        {endpoints.length > 1 && (
          <select
            className="mb-3 w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-sm"
            value={endpointId}
            onChange={(e) => setEndpointId(e.target.value)}
            aria-label="Stash-box endpoint"
          >
            {endpoints.map((ep) => (
              <option key={ep.id} value={ep.id}>
                {ep.name}
              </option>
            ))}
          </select>
        )}

        {loading ? (
          <p className="py-6 text-center text-sm text-[var(--color-muted-foreground)]">
            Loading unenriched scenes…
          </p>
        ) : (
          <>
            {rows.length === 0 && !error ? (
              <p className="py-4 text-center text-sm text-[var(--color-muted-foreground)]">
                Nothing to identify — every fingerprinted scene is already linked.
              </p>
            ) : (
              <ul className="space-y-2">
                {rows.map((row) => (
                  <li
                    key={row.scene.id}
                    className="rounded-md border border-[var(--color-border)] p-3 text-sm"
                  >
                    <p className="truncate font-medium">{row.scene.title}</p>
                    {row.state === "pending" && (
                      <p className="text-xs text-[var(--color-muted-foreground)]">Waiting…</p>
                    )}
                    {row.state === "working" && (
                      <p className="text-xs text-[var(--color-muted-foreground)]">Matching…</p>
                    )}
                    {row.state === "nomatch" && (
                      <p className="text-xs text-[var(--color-muted-foreground)]">
                        No fingerprint match — try Enrich for a title search.
                      </p>
                    )}
                    {row.state === "error" && (
                      <p className="text-xs text-red-500">{row.error ?? "Failed"}</p>
                    )}
                    {row.state === "applying" && (
                      <p className="text-xs text-[var(--color-muted-foreground)]">Applying…</p>
                    )}
                    {row.state === "applied" && (
                      <p className="flex items-center gap-1 text-xs text-green-400">
                        <Check className="h-3.5 w-3.5" /> Applied
                      </p>
                    )}
                    {(row.state === "matched" || row.state === "applying") && row.match && (
                      <div className="mt-1 flex items-start justify-between gap-2">
                        <div className="min-w-0 text-xs">
                          <p className="truncate">
                            → {row.match.title}
                            {row.match.studio ? ` · ${row.match.studio.name}` : ""}
                          </p>
                          {row.match.performers.length > 0 && (
                            <p className="truncate text-[var(--color-muted-foreground)]">
                              {row.match.performers.map((p) => p.name).join(", ")}
                            </p>
                          )}
                        </div>
                        {row.state === "matched" && (
                          <Button size="sm" variant="outline" onClick={() => void applyRow(row)}>
                            Apply
                          </Button>
                        )}
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            )}
            {total > rows.length && (
              <p className="mt-2 text-xs text-[var(--color-muted-foreground)]">
                Showing {rows.length} of {total} unenriched — repeat after applying.
              </p>
            )}
          </>
        )}

        {error && <p className="mt-3 text-sm text-red-500">{error}</p>}
        {running && (
          <p className="mt-3 text-xs text-[var(--color-muted-foreground)]">
            Matching {doneCount}/{rows.length}…
          </p>
        )}

        <div className="mt-4 flex flex-wrap items-center gap-2 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
          <div className="flex-1" />
          {running ? (
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                cancelRef.current = true;
              }}
            >
              Cancel
            </Button>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={() => void runBatch()}
              disabled={loading || rows.length === 0 || !endpointId}
            >
              <Play className="mr-1 h-3.5 w-3.5" />
              Start
            </Button>
          )}
          <Button
            size="sm"
            onClick={() => void applyAll()}
            disabled={applyingAll || matchedCount === 0}
          >
            <Check className="mr-1 h-3.5 w-3.5" />
            {applyingAll ? "Applying…" : `Apply all (${matchedCount})`}
            {appliedCount > 0 ? ` · ${appliedCount} done` : ""}
          </Button>
          <Button variant="outline" onClick={close}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
