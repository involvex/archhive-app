import { X } from "lucide-react";
import type { BrowseFilters, BrowseKind, BrowseOptions, FilterOption } from "@/lib/types";
import { cn } from "@/lib/utils";

/** Client-side duration buckets (seconds) applied to already-loaded results. */
export const LOCAL_DURATIONS = [
  { id: "", label: "Any length", min: 0, max: Infinity },
  { id: "short", label: "Under 5 min", min: 0, max: 300 },
  { id: "medium", label: "5–20 min", min: 300, max: 1200 },
  { id: "long", label: "20+ min", min: 1200, max: Infinity },
] as const;

export type LocalDurationId = (typeof LOCAL_DURATIONS)[number]["id"];

export interface LocalFilters {
  text: string;
  duration: LocalDurationId;
}

export const EMPTY_LOCAL_FILTERS: LocalFilters = { text: "", duration: "" };

/**
 * Keep items matching the on-device filters. Text matches title, channel,
 * performers and tags (`-word` excludes). Items without a known duration
 * pass the duration filter so sites that don't report it aren't emptied.
 */
export function applyLocalFilters<
  T extends {
    title: string;
    duration?: number | null;
    channel?: string | null;
    performers?: string[];
    tags?: string[];
  },
>(items: T[], f: LocalFilters): T[] {
  const words = f.text.toLowerCase().split(/\s+/).filter(Boolean);
  const include = words.filter((w) => !w.startsWith("-"));
  const exclude = words.filter((w) => w.startsWith("-") && w.length > 1).map((w) => w.slice(1));
  const bucket = LOCAL_DURATIONS.find((d) => d.id === f.duration) ?? LOCAL_DURATIONS[0];
  if (!include.length && !exclude.length && !f.duration) return items;
  return items.filter((item) => {
    const hay = [item.title, item.channel ?? "", ...(item.performers ?? []), ...(item.tags ?? [])]
      .join(" ")
      .toLowerCase();
    if (include.some((w) => !hay.includes(w))) return false;
    if (exclude.some((w) => hay.includes(w))) return false;
    if (f.duration && item.duration != null) {
      if (item.duration < bucket.min || item.duration >= bucket.max) return false;
    }
    return true;
  });
}

function FilterSelect({
  label,
  value,
  options,
  defaultLabel,
  onChange,
}: {
  label: string;
  value: string | undefined;
  options: FilterOption[];
  defaultLabel: string;
  onChange: (v: string | undefined) => void;
}) {
  if (!options.length) return null;
  return (
    <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-[var(--color-muted-foreground)]">
      {label}
      <select
        className={cn(
          "h-8 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-xs normal-case tracking-normal text-[var(--color-foreground)]",
          value && "border-[var(--color-primary)]",
        )}
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value || undefined)}
      >
        <option value="">{defaultLabel}</option>
        {options.map((o) => (
          <option key={o.id} value={o.id}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
}

interface BrowseFilterBarProps {
  options?: BrowseOptions;
  kind: BrowseKind;
  filters: BrowseFilters;
  onFiltersChange: (f: BrowseFilters) => void;
  local: LocalFilters;
  onLocalChange: (f: LocalFilters) => void;
}

/**
 * Site-side sort / filters (sent to the site, reloads results) plus on-device
 * filters over the loaded results.
 */
export function BrowseFilterBar({
  options,
  kind,
  filters,
  onFiltersChange,
  local,
  onLocalChange,
}: BrowseFilterBarProps) {
  const siteFilters = Boolean(options && options.filter_kinds.includes(kind));
  const active =
    Boolean(
      filters.sort ||
      filters.period ||
      filters.duration ||
      filters.quality ||
      filters.include ||
      filters.exclude ||
      filters.tag ||
      filters.source,
    ) || Boolean(local.text || local.duration);

  return (
    <div className="flex flex-wrap items-end gap-2">
      {siteFilters && options && (
        <>
          <FilterSelect
            label="Sort"
            value={filters.sort}
            options={options.sorts}
            defaultLabel="Site default"
            onChange={(sort) => onFiltersChange({ ...filters, sort })}
          />
          <FilterSelect
            label="Date"
            value={filters.period}
            options={options.periods}
            defaultLabel="Any time"
            onChange={(period) => onFiltersChange({ ...filters, period })}
          />
          <FilterSelect
            label="Length"
            value={filters.duration}
            options={options.durations}
            defaultLabel="Any length"
            onChange={(duration) => onFiltersChange({ ...filters, duration })}
          />
          <FilterSelect
            label="Quality"
            value={filters.quality}
            options={options.qualities}
            defaultLabel="Any quality"
            onChange={(quality) => onFiltersChange({ ...filters, quality })}
          />
          {(options.sources?.length ?? 0) > 0 && (
            <FilterSelect
              label="Source"
              value={filters.source}
              options={options.sources ?? []}
              defaultLabel="Free sites"
              onChange={(source) => onFiltersChange({ ...filters, source })}
            />
          )}
        </>
      )}
      {(!siteFilters || !options?.durations.length) && (
        <FilterSelect
          label="Length (loaded)"
          value={local.duration || undefined}
          options={LOCAL_DURATIONS.filter((d) => d.id).map((d) => ({ id: d.id, label: d.label }))}
          defaultLabel="Any length"
          onChange={(d) => onLocalChange({ ...local, duration: (d ?? "") as LocalDurationId })}
        />
      )}
      <label className="flex min-w-36 flex-1 flex-col gap-1 text-[10px] uppercase tracking-wide text-[var(--color-muted-foreground)] sm:max-w-56">
        Filter results
        <input
          className="h-8 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-xs normal-case tracking-normal text-[var(--color-foreground)]"
          placeholder="words, -exclude"
          value={local.text}
          onChange={(e) => onLocalChange({ ...local, text: e.target.value })}
        />
      </label>
      {active && (
        <button
          type="button"
          className="mb-1 inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
          onClick={() => {
            onFiltersChange({});
            onLocalChange(EMPTY_LOCAL_FILTERS);
          }}
        >
          <X className="h-3 w-3" /> Reset
        </button>
      )}
    </div>
  );
}
