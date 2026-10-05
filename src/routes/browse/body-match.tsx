import { Link, createFileRoute } from "@tanstack/react-router";
import { useCallback, useState } from "react";
import { ExternalLink, History, Library, Search } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api/client";
import { openExternal } from "@/lib/external";
import type { Performer, Scene } from "@/lib/types";
import {
  buildBodyMatchLinks,
  CUP_OPTIONS,
  HAIR_OPTIONS,
  type BodyMatchLink,
  type CupId,
  type HairId,
} from "@/lib/body-match/queries";

export const Route = createFileRoute("/browse/body-match")({
  component: BodyMatchPage,
});

const RECENT_KEY = "bodymatch:recent";
const RECENT_MAX = 10;

interface RecentEntry {
  cup: CupId;
  hair: HairId;
  at: string;
}

function loadRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? (parsed as RecentEntry[]).slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

interface LibraryMatches {
  performers: Performer[];
  scenes: Scene[];
}

function BodyMatchPage() {
  const [cup, setCup] = useState<CupId>("DD");
  const [hair, setHair] = useState<HairId>("brunette");
  const [links, setLinks] = useState<BodyMatchLink[] | null>(null);
  const [recent, setRecent] = useState<RecentEntry[]>(loadRecent);
  const [library, setLibrary] = useState<LibraryMatches | null>(null);
  const [libraryLoading, setLibraryLoading] = useState(false);
  const [libraryError, setLibraryError] = useState("");

  const loadLibrary = useCallback((c: CupId, h: HairId) => {
    setLibraryLoading(true);
    setLibraryError("");
    const performer_hair = h === "any" ? undefined : h;
    void Promise.all([
      api
        .listPerformers(undefined, {
          cup_size: c,
          ...(performer_hair ? { hair_color: performer_hair } : {}),
        })
        .catch(() => [] as Performer[]),
      api
        .listScenesWithFilter({
          performer_cup: c,
          ...(performer_hair ? { performer_hair } : {}),
        })
        .catch(() => [] as Scene[]),
    ])
      .then(([performers, scenes]) => setLibrary({ performers, scenes }))
      .catch((e) => setLibraryError(e instanceof Error ? e.message : "Library search failed"))
      .finally(() => setLibraryLoading(false));
  }, []);

  const handleSearch = useCallback(() => {
    const built = buildBodyMatchLinks({ cup, hair });
    setLinks(built);
    loadLibrary(cup, hair);
    setRecent((prev) => {
      const next = [{ cup, hair, at: new Date().toISOString() }, ...prev].slice(0, RECENT_MAX);
      try {
        localStorage.setItem(RECENT_KEY, JSON.stringify(next));
      } catch {
        /* ignore */
      }
      return next;
    });
  }, [cup, hair, loadLibrary]);

  const applyRecent = useCallback(
    (entry: RecentEntry) => {
      setCup(entry.cup);
      setHair(entry.hair);
      setLinks(buildBodyMatchLinks({ cup: entry.cup, hair: entry.hair }));
      loadLibrary(entry.cup, entry.hair);
    },
    [loadLibrary],
  );

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Cup &amp; Color Finder</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Filter by cup size and hair color — library matches plus external site searches.
        </p>
      </div>

      <Card>
        <CardContent className="space-y-3 p-4">
          <div className="flex flex-col gap-2 sm:flex-row">
            <label className="flex flex-1 flex-col gap-1 text-xs font-medium text-[var(--color-muted-foreground)]">
              Cup size
              <select
                className="h-10 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 text-sm text-[var(--color-foreground)]"
                value={cup}
                onChange={(e) => setCup(e.target.value as CupId)}
              >
                {CUP_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex flex-1 flex-col gap-1 text-xs font-medium text-[var(--color-muted-foreground)]">
              Hair color
              <select
                className="h-10 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 text-sm text-[var(--color-foreground)]"
                value={hair}
                onChange={(e) => setHair(e.target.value as HairId)}
              >
                {HAIR_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </label>
            <div className="flex items-end">
              <Button onClick={handleSearch} className="h-10 shrink-0">
                <Search className="h-4 w-4 sm:mr-2" />
                <span className="hidden sm:inline">Search</span>
              </Button>
            </div>
          </div>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Queries are English (e.g. &ldquo;brunette DD cup&rdquo;). Google uses an expanded
            OR-query for DD&ndash;H+. Library matching needs cup/hair set on the performer (Library
            → Performers → detail page).
          </p>
        </CardContent>
      </Card>

      {links && (
        <Card>
          <CardContent className="space-y-3 p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Library className="h-4 w-4" />
                <span>
                  Library matches
                  {library && (
                    <span className="ml-2 text-xs text-[var(--color-muted-foreground)]">
                      {library.performers.length} performers · {library.scenes.length} scenes
                    </span>
                  )}
                </span>
              </div>
              <Button asChild size="sm" variant="outline">
                <Link
                  to="/library/scenes"
                  search={{
                    cup,
                    ...(hair === "any" ? {} : { hair }),
                  }}
                >
                  Open in Library
                </Link>
              </Button>
            </div>
            {libraryLoading && (
              <p className="text-xs text-[var(--color-muted-foreground)]">Searching library…</p>
            )}
            {libraryError && <p className="text-xs text-red-400">{libraryError}</p>}
            {library && !libraryLoading && (
              <>
                {library.performers.length === 0 && library.scenes.length === 0 ? (
                  <p className="text-xs text-[var(--color-muted-foreground)]">
                    No library matches yet — set cup/hair on performers to match them here.
                  </p>
                ) : (
                  <div className="space-y-2">
                    {library.performers.slice(0, 6).map((p) => (
                      <Link
                        key={p.id}
                        to="/library/performers/$performerId"
                        params={{ performerId: p.id }}
                        className="flex items-center justify-between rounded-md px-3 py-2 text-sm hover:bg-[var(--color-muted)]"
                      >
                        <span className="font-medium">{p.name}</span>
                        <span className="text-xs text-[var(--color-muted-foreground)]">
                          {[p.cup_size ?? null, p.hair_color ?? null].filter(Boolean).join(" · ")}
                          {` · ${p.scene_count} scenes`}
                        </span>
                      </Link>
                    ))}
                    {library.scenes.length > 0 && (
                      <p className="truncate text-xs text-[var(--color-muted-foreground)]">
                        Scenes:{" "}
                        {library.scenes
                          .slice(0, 5)
                          .map((s) => s.title)
                          .join(" · ")}
                        {library.scenes.length > 5 && ` (+${library.scenes.length - 5} more)`}
                      </p>
                    )}
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>
      )}

      {links && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {links.map((link) => (
            <Card key={link.site}>
              <CardContent className="flex flex-col gap-2 p-4">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold">{link.label}</span>
                  <ExternalLink className="h-4 w-4 text-[var(--color-muted-foreground)]" />
                </div>
                <code className="truncate rounded bg-[var(--color-secondary)] px-2 py-1 text-xs text-[var(--color-secondary-foreground)]">
                  {link.query}
                </code>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-1"
                  onClick={() => void openExternal(link.url).catch(console.error)}
                >
                  Open search
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {recent.length > 0 && (
        <Card>
          <CardContent className="space-y-2 p-4">
            <div className="flex items-center gap-2 text-sm font-medium">
              <History className="h-4 w-4" />
              Recent
            </div>
            <div className="flex flex-wrap gap-1.5">
              {recent.map((r, i) => (
                <button
                  key={`${r.cup}-${r.hair}-${r.at}-${i}`}
                  type="button"
                  onClick={() => applyRecent(r)}
                  className="rounded-full bg-[var(--color-secondary)] px-3 py-1 text-xs hover:bg-[var(--color-muted)]"
                >
                  {r.cup} · {r.hair}
                </button>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
