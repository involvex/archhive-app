import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene, StashBoxEndpoint, StashSceneMatch } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Check, Search, Upload, X } from "lucide-react";

interface EnrichDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
  onApplied: (updated: Scene) => void;
}

interface FieldFlags {
  title: boolean;
  date: boolean;
  studio: boolean;
  performers: boolean;
  tags: boolean;
  cover: boolean;
}

const DEFAULT_FLAGS: FieldFlags = {
  title: true,
  date: true,
  studio: true,
  performers: true,
  tags: true,
  cover: false,
};

/**
 * Filename-derived titles look like "TWISTYS - Watch Runaway Bride …".
 * StashDB stores just "Runaway Bride", so strip a leading "STUDIO - "
 * prefix and a leading "Watch " for the first title search. The field
 * stays editable — this only improves the first attempt.
 */
export function suggestSearchTitle(raw: string): string {
  let t = raw.trim();
  t = t.replace(/^.+?\s[-–—]\s+/, "");
  t = t.replace(/^watch\s+/i, "");
  return t.trim() || raw.trim();
}

/**
 * Tagger-lite: match one library scene against StashDB and apply the chosen
 * fields. Fingerprint-first (MD5/OSHASH), title search as fallback.
 * Performers/tags merge — nothing already on the scene is removed.
 */
export function EnrichDialog({ scene, open, onClose, onApplied }: EnrichDialogProps) {
  const [endpoints, setEndpoints] = useState<StashBoxEndpoint[]>([]);
  const [endpointId, setEndpointId] = useState<string>("");
  const [candidates, setCandidates] = useState<StashSceneMatch[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [flags, setFlags] = useState<FieldFlags>(DEFAULT_FLAGS);
  const [queryTitle, setQueryTitle] = useState("");
  const [querying, setQuerying] = useState(false);
  const [applying, setApplying] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [applied, setApplied] = useState(false);

  const resetForScene = useCallback((s: Scene) => {
    setCandidates([]);
    setSelectedId(null);
    setFlags(DEFAULT_FLAGS);
    setQueryTitle(suggestSearchTitle(s.title));
    setError(null);
    setStatus("");
    setApplied(false);
  }, []);

  useEffect(() => {
    if (!open || !scene) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    resetForScene(scene);
    let cancelled = false;
    void (async () => {
      try {
        const eps = await api.listStashboxEndpoints();
        if (cancelled) return;
        setEndpoints(eps);
        const first = eps[0]?.id ?? "";
        setEndpointId(first);
        if (eps.length === 0) {
          setError("No stash-box endpoint configured — add one in Settings → Metadata.");
          return;
        }
        setQuerying(true);
        const matches = await api.queryStashdbForScene(scene.id, first || undefined);
        if (cancelled) return;
        setCandidates(matches);
        setSelectedId(matches[0]?.stash_id ?? null);
        if (matches.length === 0) {
          setStatus("No fingerprint match — try a title search below.");
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Query failed");
      } finally {
        if (!cancelled) setQuerying(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, scene, resetForScene]);

  async function runTitleSearch() {
    if (!scene || querying || !queryTitle.trim()) return;
    setQuerying(true);
    setError(null);
    setStatus("");
    try {
      const matches = await api.searchStashdbScenes(queryTitle.trim(), endpointId || undefined);
      setCandidates(matches);
      setSelectedId(matches[0]?.stash_id ?? null);
      if (matches.length === 0) setStatus("No matches for that title.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Search failed");
    } finally {
      setQuerying(false);
    }
  }

  async function applySelected() {
    if (!scene || !selectedId || applying) return;
    setApplying(true);
    setError(null);
    try {
      const updated = await api.applyStashdbMatch({
        scene_id: scene.id,
        endpoint_id: endpointId,
        stash_id: selectedId,
        apply_title: flags.title,
        apply_date: flags.date,
        apply_studio: flags.studio,
        apply_performers: flags.performers,
        apply_tags: flags.tags,
        apply_image_as_thumb: flags.cover,
      });
      setApplied(true);
      setStatus("Applied. You can submit fingerprints so others can match this file.");
      onApplied(updated);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Apply failed");
    } finally {
      setApplying(false);
    }
  }

  async function submitFingerprints() {
    if (!scene || submitting) return;
    setSubmitting(true);
    try {
      const result = await api.submitStashdbFingerprints(scene.id, endpointId || undefined);
      setStatus(`Submitted ${result.submitted} fingerprint(s) — thanks!`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Submit failed");
    } finally {
      setSubmitting(false);
    }
  }

  if (!open || !scene) return null;
  const selected = candidates.find((c) => c.stash_id === selectedId) ?? null;

  function toggleFlag(key: keyof FieldFlags) {
    setFlags((prev) => ({ ...prev, [key]: !prev[key] }));
  }

  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Enrich from StashDB"
        className="max-h-[92dvh] w-full max-w-lg overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <div className="mb-3 flex items-center justify-between gap-2">
          <h3 className="text-lg font-semibold">Enrich from StashDB</h3>
          <button
            type="button"
            onClick={onClose}
            className="shrink-0 rounded p-1 hover:bg-[var(--color-muted)]"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <p className="mb-3 line-clamp-2 text-sm text-[var(--color-muted-foreground)]">
          {scene.title}
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

        {querying && candidates.length === 0 ? (
          <p className="py-6 text-center text-sm text-[var(--color-muted-foreground)]">
            Matching fingerprints…
          </p>
        ) : (
          <>
            {candidates.length > 0 ? (
              <ul className="space-y-2">
                {candidates.map((c) => (
                  <li key={c.stash_id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(c.stash_id)}
                      className={`w-full rounded-md border p-3 text-left text-sm ${
                        c.stash_id === selectedId
                          ? "border-[var(--color-primary)]"
                          : "border-[var(--color-border)]"
                      }`}
                    >
                      <span className="flex items-start gap-2">
                        <span
                          className={`mt-1 flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${
                            c.stash_id === selectedId
                              ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                              : "border-[var(--color-muted-foreground)]"
                          }`}
                        >
                          {c.stash_id === selectedId && <Check className="h-3 w-3" />}
                        </span>
                        <span className="min-w-0">
                          <span className="block truncate font-medium">{c.title}</span>
                          <span className="block text-xs text-[var(--color-muted-foreground)]">
                            {[c.date, c.studio?.name].filter(Boolean).join(" · ") ||
                              "no date/studio"}
                            {c.duration ? ` · ${Math.round(c.duration / 60)} min` : ""}
                          </span>
                          {c.performers.length > 0 && (
                            <span className="block truncate text-xs">
                              {c.performers.map((p) => p.name).join(", ")}
                            </span>
                          )}
                        </span>
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              !error && (
                <p className="text-sm text-[var(--color-muted-foreground)]">
                  No matches yet — try a title search.
                </p>
              )
            )}

            <div className="mt-3 flex gap-2">
              <Input
                placeholder="Search by title…"
                value={queryTitle}
                onChange={(e) => setQueryTitle(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void runTitleSearch();
                }}
              />
              <Button variant="outline" onClick={() => void runTitleSearch()} disabled={querying}>
                <Search className="h-4 w-4" />
              </Button>
            </div>

            {selected && (
              <div className="mt-3 space-y-2 rounded-md border border-[var(--color-border)] p-3 text-sm">
                <p className="text-xs font-medium text-[var(--color-muted-foreground)]">
                  Apply from “{selected.title}”
                </p>
                {(
                  [
                    ["title", `Title: ${selected.title}`],
                    ["date", `Date: ${selected.date ?? "—"}`],
                    ["studio", `Studio: ${selected.studio?.name ?? "—"}`],
                    [
                      "performers",
                      `Performers: ${selected.performers.map((p) => p.name).join(", ") || "—"}`,
                    ],
                    ["tags", `Tags: ${selected.tags.map((t) => t.name).join(", ") || "—"}`],
                    ["cover", `Cover image${selected.image ? "" : " (none available)"}`],
                  ] as [keyof FieldFlags, string][]
                ).map(([key, label]) => (
                  <label key={key} className="flex items-start gap-2 text-sm">
                    <input
                      type="checkbox"
                      className="mt-1"
                      checked={flags[key]}
                      disabled={key === "cover" && !selected.image}
                      onChange={() => toggleFlag(key)}
                    />
                    <span className="min-w-0 break-words">{label}</span>
                  </label>
                ))}
                {selected.details && (
                  <p className="line-clamp-3 text-xs text-[var(--color-muted-foreground)]">
                    {selected.details}
                  </p>
                )}
              </div>
            )}
          </>
        )}

        {error && <p className="mt-3 text-sm text-red-500">{error}</p>}
        {status && !error && (
          <p className="mt-3 text-xs text-[var(--color-muted-foreground)]">{status}</p>
        )}

        <div className="mt-4 flex flex-wrap items-center gap-2 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
          <div className="flex-1" />
          {applied && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => void submitFingerprints()}
              disabled={submitting}
            >
              <Upload className="mr-1 h-3.5 w-3.5" />
              {submitting ? "Submitting…" : "Submit fingerprints"}
            </Button>
          )}
          <Button size="sm" onClick={() => void applySelected()} disabled={!selected || applying}>
            {applying ? "Applying…" : "Apply"}
          </Button>
          <Button variant="outline" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
