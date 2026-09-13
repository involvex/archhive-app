import { Input } from "@/components/ui/input";
import type { SceneFilter } from "@/lib/types";
import { X } from "lucide-react";

interface FilterPillProps {
  label: string;
  onRemove: () => void;
}

function FilterPill({ label, onRemove }: FilterPillProps) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-[var(--color-border)] bg-[var(--color-card)] px-2 py-0.5 text-xs">
      {label}
      <button
        type="button"
        onClick={onRemove}
        className="hover:text-red-400"
        aria-label="Remove filter"
      >
        <X className="h-3 w-3" />
      </button>
    </span>
  );
}

interface FilterBuilderProps {
  filter: SceneFilter;
  onChange: (filter: SceneFilter | ((prev: SceneFilter) => SceneFilter)) => void;
  performerInput?: string;
  onPerformerInputChange?: (value: string) => void;
  tagInput?: string;
  onTagInputChange?: (value: string) => void;
  showDurationInputs?: boolean;
  showFileSizeInput?: boolean;
  showPerformerTagInputs?: boolean;
  showRatingFilters?: boolean;
  onClearAll?: () => void;
}

export function FilterBuilder({
  filter,
  onChange,
  performerInput = "",
  onPerformerInputChange,
  tagInput = "",
  onTagInputChange,
  showDurationInputs = true,
  showFileSizeInput = true,
  showPerformerTagInputs = true,
  showRatingFilters = true,
  onClearAll,
}: FilterBuilderProps) {
  const hasFilter =
    filter.missing_thumb ||
    filter.missing_duration ||
    filter.hash_named ||
    filter.hide_watched ||
    filter.min_duration != null ||
    filter.max_duration != null ||
    filter.min_rating != null ||
    filter.min_file_size != null ||
    (filter.performer_names?.length ?? 0) > 0 ||
    (filter.tag_names?.length ?? 0) > 0;

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1.5">
        {[
          { key: "missing_thumb", label: "Missing thumb" },
          { key: "missing_duration", label: "Missing duration" },
          { key: "short", label: "≤ 15s" },
          { key: "hash_named", label: "Hash-named" },
          { key: "hide_watched", label: "Hide watched" },
          ...(showRatingFilters
            ? [
                { key: "rating-3", label: "★★★+" },
                { key: "rating-4", label: "★★★★+" },
                { key: "rating-5", label: "★★★★★" },
              ]
            : []),
        ].map(({ key, label }) => {
          const active =
            key === "short"
              ? filter.max_duration === 15
              : key.startsWith("rating-")
                ? filter.min_rating === Number(key.slice(7))
                : !!filter[key as keyof SceneFilter];
          return (
            <button
              key={key}
              type="button"
              onClick={() => {
                if (key === "short")
                  onChange((f) => ({
                    ...f,
                    max_duration: f.max_duration === 15 ? undefined : 15,
                  }));
                else if (key === "rating-3") {
                  if (filter.min_rating === 3) onChange((f) => ({ ...f, min_rating: undefined }));
                  else onChange((f) => ({ ...f, min_rating: 3 }));
                } else if (key === "rating-4") {
                  if (filter.min_rating === 4) onChange((f) => ({ ...f, min_rating: undefined }));
                  else onChange((f) => ({ ...f, min_rating: 4 }));
                } else if (key === "rating-5") {
                  if (filter.min_rating === 5) onChange((f) => ({ ...f, min_rating: undefined }));
                  else onChange((f) => ({ ...f, min_rating: 5 }));
                } else {
                  onChange((f) => ({ ...f, [key]: !f[key as keyof SceneFilter] }));
                }
              }}
              className={`rounded-full px-3 py-1 text-xs font-medium transition ${
                active
                  ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                  : "bg-[var(--color-secondary)] text-[var(--color-secondary-foreground)] hover:bg-[var(--color-muted)]"
              }`}
            >
              {label}
            </button>
          );
        })}
      </div>

      {showDurationInputs && (
        <div className="flex items-center gap-1">
          <Input
            type="number"
            placeholder="Min s"
            value={filter.min_duration ?? ""}
            onChange={(e) => {
              const v = e.target.value ? Number(e.target.value) : undefined;
              onChange((f) => ({ ...f, min_duration: v }));
            }}
            className="h-7 w-16 text-xs"
            aria-label="Minimum duration in seconds"
          />
          <span className="text-xs text-[var(--color-muted-foreground)]">–</span>
          <Input
            type="number"
            placeholder="Max s"
            value={filter.max_duration ?? ""}
            onChange={(e) => {
              const v = e.target.value ? Number(e.target.value) : undefined;
              onChange((f) => ({ ...f, max_duration: v }));
            }}
            className="h-7 w-16 text-xs"
            aria-label="Maximum duration in seconds"
          />
        </div>
      )}

      {showFileSizeInput && (
        <div className="flex items-center gap-1">
          <Input
            type="number"
            placeholder="Min size (MB)"
            value={filter.min_file_size ? Math.round(filter.min_file_size / 1048576) : ""}
            onChange={(e) => {
              const mb = e.target.value ? Number(e.target.value) : 0;
              onChange((f) => ({
                ...f,
                min_file_size: mb > 0 ? mb * 1048576 : undefined,
              }));
            }}
            className="h-7 w-24 text-xs"
            aria-label="Minimum file size in MB"
          />
        </div>
      )}

      {showPerformerTagInputs && (
        <>
          {(filter.performer_names?.length ?? 0) > 0 ? (
            filter.performer_names!.map((name) => (
              <FilterPill
                key={name}
                label={`Performer: ${name}`}
                onRemove={() => {
                  onChange((f) => {
                    const names = (f.performer_names ?? []).filter((n) => n !== name);
                    return { ...f, performer_names: names.length > 0 ? names : undefined };
                  });
                  onPerformerInputChange?.(
                    filter.performer_names!.filter((n) => n !== name).join(", "),
                  );
                }}
              />
            ))
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const v = performerInput.trim();
                if (v) {
                  const names = [...(filter.performer_names ?? []), v];
                  onChange((f) => ({ ...f, performer_names: names }));
                  onPerformerInputChange?.("");
                }
              }}
              className="flex items-center gap-1"
            >
              <Input
                placeholder="Filter performer…"
                value={performerInput}
                onChange={(e) => onPerformerInputChange?.(e.target.value)}
                className="h-7 w-36 text-xs"
              />
            </form>
          )}

          {(filter.tag_names?.length ?? 0) > 0 ? (
            filter.tag_names!.map((name) => (
              <FilterPill
                key={name}
                label={`Tag: ${name}`}
                onRemove={() => {
                  onChange((f) => {
                    const names = (f.tag_names ?? []).filter((n) => n !== name);
                    return { ...f, tag_names: names.length > 0 ? names : undefined };
                  });
                  onTagInputChange?.(filter.tag_names!.filter((n) => n !== name).join(", "));
                }}
              />
            ))
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const v = tagInput.trim();
                if (v) {
                  const names = [...(filter.tag_names ?? []), v];
                  onChange((f) => ({ ...f, tag_names: names }));
                  onTagInputChange?.("");
                }
              }}
              className="flex items-center gap-1"
            >
              <Input
                placeholder="Filter tag…"
                value={tagInput}
                onChange={(e) => onTagInputChange?.(e.target.value)}
                className="h-7 w-36 text-xs"
              />
            </form>
          )}
        </>
      )}

      {filter.min_rating != null && showRatingFilters && (
        <FilterPill
          label={`★ ${filter.min_rating}+`}
          onRemove={() => onChange((f) => ({ ...f, min_rating: undefined }))}
        />
      )}

      {filter.min_file_size != null && showFileSizeInput && (
        <FilterPill
          label={`≥ ${Math.round(filter.min_file_size / 1048576)}MB`}
          onRemove={() => onChange((f) => ({ ...f, min_file_size: undefined }))}
        />
      )}

      {hasFilter && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs text-[var(--color-muted-foreground)]">Active filters:</span>
          {filter.missing_thumb && (
            <FilterPill
              label="Missing thumb"
              onRemove={() => onChange((f) => ({ ...f, missing_thumb: false }))}
            />
          )}
          {filter.missing_duration && (
            <FilterPill
              label="Missing duration"
              onRemove={() => onChange((f) => ({ ...f, missing_duration: false }))}
            />
          )}
          {filter.hash_named && (
            <FilterPill
              label="Hash-named"
              onRemove={() => onChange((f) => ({ ...f, hash_named: false }))}
            />
          )}
          {filter.max_duration != null && (
            <FilterPill
              label={`≤ ${filter.max_duration}s`}
              onRemove={() => onChange((f) => ({ ...f, max_duration: undefined }))}
            />
          )}
          {filter.min_duration != null && (
            <FilterPill
              label={`≥ ${filter.min_duration}s`}
              onRemove={() => onChange((f) => ({ ...f, min_duration: undefined }))}
            />
          )}
          {(filter.performer_names?.length ?? 0) > 0
            ? filter.performer_names!.map((name) => (
                <FilterPill
                  key={name}
                  label={`Performer: ${name}`}
                  onRemove={() => {
                    onChange((f) => {
                      const names = (f.performer_names ?? []).filter((n) => n !== name);
                      return { ...f, performer_names: names.length > 0 ? names : undefined };
                    });
                    onPerformerInputChange?.(
                      filter.performer_names!.filter((n) => n !== name).join(", "),
                    );
                  }}
                />
              ))
            : null}
          {(filter.tag_names?.length ?? 0) > 0
            ? filter.tag_names!.map((name) => (
                <FilterPill
                  key={name}
                  label={`Tag: ${name}`}
                  onRemove={() => {
                    onChange((f) => {
                      const names = (f.tag_names ?? []).filter((n) => n !== name);
                      return { ...f, tag_names: names.length > 0 ? names : undefined };
                    });
                    onTagInputChange?.(filter.tag_names!.filter((n) => n !== name).join(", "));
                  }}
                />
              ))
            : null}
          {filter.min_rating != null && (
            <FilterPill
              label={`★ ${filter.min_rating}+`}
              onRemove={() => onChange((f) => ({ ...f, min_rating: undefined }))}
            />
          )}
          {filter.min_file_size != null && (
            <FilterPill
              label={`≥ ${Math.round(filter.min_file_size / 1048576)}MB`}
              onRemove={() => onChange((f) => ({ ...f, min_file_size: undefined }))}
            />
          )}
          {onClearAll && (
            <button
              type="button"
              onClick={onClearAll}
              className="text-xs text-[var(--color-muted-foreground)] underline hover:text-[var(--color-foreground)]"
            >
              Clear all
            </button>
          )}
        </div>
      )}
    </div>
  );
}
