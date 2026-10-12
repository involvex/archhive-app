import { useEffect, useMemo, useState } from "react";
import { Check, ChevronDown, Search, Sparkles, X } from "lucide-react";
import type { SiteInfo } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/** Stable per-site hue so each site gets its own avatar colour. */
function siteHue(id: string): number {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return h;
}

/**
 * Sites with a bundled icon in `public/site-icons/<id>.png` (128×128, taken
 * from each site's apple-touch / largest favicon). Sites whose only icon is
 * tiny (ThotHub, ThisVid) and plugin sites keep the initial badge.
 */
const SITE_ICONS = new Set([
  "chaturbate",
  "stripchat",
  "pornhub",
  "youporn",
  "xnxx",
  "xhamster",
  "xvideos",
  "reddit",
  "redgifs",
  "eporner",
  "youtube",
  "tiktok",
  "instagram",
  "twitter",
]);

export function SiteAvatar({ site, size = "md" }: { site?: SiteInfo; size?: "sm" | "md" }) {
  const [broken, setBroken] = useState(false);
  const cls = size === "sm" ? "h-6 w-6 text-[11px]" : "h-10 w-10 text-sm";
  if (site && SITE_ICONS.has(site.id) && !broken) {
    return (
      <img
        src={`/site-icons/${site.id}.png`}
        alt=""
        className={cn("shrink-0 rounded-lg object-contain", cls)}
        onError={() => setBroken(true)}
        draggable={false}
      />
    );
  }
  if (!site) {
    return (
      <span
        className={cn(
          "flex shrink-0 items-center justify-center rounded-full bg-[var(--color-primary)] text-[var(--color-primary-foreground)]",
          cls,
        )}
      >
        <Sparkles className={size === "sm" ? "h-3.5 w-3.5" : "h-5 w-5"} />
      </span>
    );
  }
  const hue = siteHue(site.id);
  const initials = site.display_name
    .replace(/[^A-Za-z0-9 ]/g, "")
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  return (
    <span
      className={cn("flex shrink-0 items-center justify-center rounded-full font-bold", cls)}
      style={{ background: `oklch(0.42 0.12 ${hue})`, color: `oklch(0.95 0.03 ${hue})` }}
      aria-hidden
    >
      {initials || "?"}
    </span>
  );
}

interface SitePickerProps {
  sites: SiteInfo[];
  /** Selected site id, or "auto" to detect the site from the query. */
  value: string;
  onChange: (siteId: string) => void;
  className?: string;
}

/**
 * Site selector: one large button showing the current site, opening a grid of
 * big tiles (a bottom sheet on phones). Replaces the tiny sideways-scrolling
 * chip row plus the separate dropdown.
 */
export function SitePicker({ sites, value, onChange, className }: SitePickerProps) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const current = sites.find((s) => s.id === value);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return q ? sites.filter((s) => s.display_name.toLowerCase().includes(q)) : sites;
  }, [sites, filter]);

  function choose(id: string) {
    onChange(id);
    setOpen(false);
    setFilter("");
  }

  const tile = (selected: boolean) =>
    cn(
      "relative flex min-h-16 items-center gap-2.5 rounded-xl p-2.5 text-left transition active:scale-[0.97]",
      selected
        ? "bg-[var(--color-primary)]/15 ring-2 ring-[var(--color-primary)]"
        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]",
    );

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        aria-haspopup="dialog"
        className={cn(
          "flex h-10 items-center gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2.5 text-sm",
          className,
        )}
      >
        <SiteAvatar site={current} size="sm" />
        <span className="min-w-0 flex-1 truncate text-left font-medium">
          {current?.display_name ?? "Auto-detect site"}
        </span>
        <ChevronDown className="h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]" />
      </button>

      {open && (
        <div
          className="fixed inset-0 z-50"
          role="dialog"
          aria-modal="true"
          aria-label="Choose site"
        >
          <button
            type="button"
            aria-label="Close site picker"
            className="fixed inset-0 cursor-default bg-black/60"
            onClick={() => setOpen(false)}
          />
          <div className="fixed right-0 bottom-0 left-0 flex max-h-[85dvh] flex-col overflow-hidden rounded-t-2xl border-t border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl md:top-1/2 md:right-auto md:bottom-auto md:left-1/2 md:w-[min(42rem,90vw)] md:-translate-x-1/2 md:-translate-y-1/2 md:rounded-2xl md:border">
            <div className="mx-auto mt-2 h-1 w-10 rounded-full bg-[var(--color-muted-foreground)] opacity-40 md:hidden" />
            <div className="flex items-center gap-2 px-4 pt-3 pb-2">
              <p className="flex-1 text-sm font-semibold">Choose a site</p>
              <button
                type="button"
                onClick={() => setOpen(false)}
                aria-label="Close"
                className="rounded-full p-1.5 hover:bg-[var(--color-muted)]"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
            {sites.length > 8 && (
              <div className="relative px-4 pb-2">
                <Search className="absolute top-2.5 left-6 h-4 w-4 text-[var(--color-muted-foreground)]" />
                <Input
                  className="pl-8"
                  placeholder="Find a site…"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                />
              </div>
            )}
            <div className="grid grid-cols-2 gap-2 overflow-y-auto overscroll-contain px-4 pb-[max(1rem,env(safe-area-inset-bottom))] sm:grid-cols-3">
              {!filter && (
                <button
                  type="button"
                  onClick={() => choose("auto")}
                  className={tile(value === "auto")}
                >
                  <SiteAvatar />
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">Auto-detect</span>
                    <span className="block text-[11px] text-[var(--color-muted-foreground)]">
                      From the URL or query
                    </span>
                  </span>
                </button>
              )}
              {visible.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => choose(s.id)}
                  className={tile(value === s.id)}
                >
                  <SiteAvatar site={s} />
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">{s.display_name}</span>
                    {s.requires_cookies && (
                      <span className="block text-[11px] text-[var(--color-muted-foreground)]">
                        Needs cookies
                      </span>
                    )}
                  </span>
                  {value === s.id && (
                    <Check className="absolute top-1.5 right-1.5 h-4 w-4 text-[var(--color-primary)]" />
                  )}
                </button>
              ))}
              {visible.length === 0 && (
                <p className="col-span-full py-4 text-center text-xs text-[var(--color-muted-foreground)]">
                  No site matches “{filter}”.
                </p>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
