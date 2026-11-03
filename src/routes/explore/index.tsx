import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { ErrorState } from "@/components/ErrorState";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { EmptyState } from "@/components/EmptyState";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Search, Shuffle, ScanFace, X, Compass } from "lucide-react";
import { cn } from "@/lib/utils";
import { useMediaQuery } from "@/lib/hooks/useMediaQuery";

export const Route = createFileRoute("/explore/")({
  component: ExploreIndexPage,
});

type Mode = "search" | "face" | "random";

const PLATFORMS = [
  { id: "", label: "All" },
  { id: "cb", label: "Chaturbate" },
  { id: "sc", label: "Stripchat" },
] as const;

const GENDERS = [
  { id: "", label: "Any" },
  { id: "f", label: "Female" },
  { id: "m", label: "Male" },
  { id: "c", label: "Couple" },
  { id: "t", label: "Trans" },
] as const;

function platformFromItem(item: MediaItem): string {
  const tag = item.tags.find((t) => t.startsWith("platform:"));
  return tag?.slice("platform:".length) ?? "cb";
}

function liveSiteForPlatform(platform: string): "chaturbate" | "stripchat" | null {
  if (platform === "cb") return "chaturbate";
  if (platform === "sc") return "stripchat";
  return null;
}

function ExploreIndexPage() {
  const [mode, setMode] = useState<Mode>("search");
  const [items, setItems] = useState<MediaItem[]>([]);
  const [query, setQuery] = useState("");
  const [platform, setPlatform] = useState("");
  const [gender, setGender] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const isPhone = useMediaQuery("(max-width: 767px)");

  const filterSlug = [platform && `platform:${platform}`, gender && `gender:${gender}`]
    .filter(Boolean)
    .join(",");

  const loadRandom = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const result = await api.browse("camgirlfinder", "livestream", filterSlug, 1);
      setItems(result.items);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Failed to load models";
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, [filterSlug]);

  const loadSearch = useCallback(async () => {
    const q = query.trim();
    if (q.length < 3) {
      setError("Enter at least 3 characters");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const result = await api.browse("camgirlfinder", "search", q, 1);
      let list = result.items;
      if (platform) {
        list = list.filter((i) => platformFromItem(i) === platform);
      }
      if (gender) {
        list = list.filter((i) => i.tags.some((t) => t === `gender:${gender}`));
      }
      setItems(list);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Search failed";
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, [query, platform, gender]);

  const runFaceSearch = useCallback(async (bytes: ArrayBuffer) => {
    setLoading(true);
    setError("");
    setMode("face");
    try {
      const u8 = new Uint8Array(bytes);
      let binary = "";
      const chunk = 0x8000;
      for (let i = 0; i < u8.length; i += chunk) {
        binary += String.fromCharCode(...u8.subarray(i, i + chunk));
      }
      const b64 = btoa(binary);
      const results = await api.camgirlfinderFaceSearch(b64);
      setItems(results);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Face search failed";
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (mode === "random") {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- load on mode enter
      void loadRandom();
    }
  }, [mode, loadRandom]);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Compass className="h-5 w-5 text-[var(--color-primary)]" />
        <h2 className="text-2xl font-bold">Explore</h2>
      </div>
      <p className="text-sm text-[var(--color-muted-foreground)]">
        Find cam accounts via CamGirlFinder (name, face, or random). Not a live grid — open
        Chaturbate / Stripchat rooms when available.
      </p>

      <div className="flex flex-wrap gap-2">
        {(
          [
            { id: "search" as const, label: "Search", icon: Search },
            { id: "face" as const, label: "Face", icon: ScanFace },
            { id: "random" as const, label: "Random", icon: Shuffle },
          ] as const
        ).map((m) => (
          <button
            key={m.id}
            type="button"
            onClick={() => {
              setMode(m.id);
              setError("");
              if (m.id !== "random") setItems([]);
            }}
            className={cn(
              "inline-flex min-h-10 items-center gap-1.5 rounded-lg px-3 text-sm font-medium",
              mode === m.id
                ? "bg-[var(--color-primary)]/15 ring-2 ring-[var(--color-primary)]"
                : "bg-[var(--color-secondary)]",
            )}
          >
            <m.icon className="h-4 w-4" />
            {m.label}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap gap-2">
        {PLATFORMS.map((p) => (
          <button
            key={p.id || "all"}
            type="button"
            onClick={() => setPlatform(p.id)}
            className={cn(
              "rounded-md px-2.5 py-1 text-xs font-medium",
              platform === p.id
                ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                : "bg-[var(--color-muted)]",
            )}
          >
            {p.label}
          </button>
        ))}
        <span className="mx-1 self-center text-[var(--color-muted-foreground)]">·</span>
        {GENDERS.map((g) => (
          <button
            key={g.id || "any"}
            type="button"
            onClick={() => setGender(g.id)}
            className={cn(
              "rounded-md px-2.5 py-1 text-xs font-medium",
              gender === g.id
                ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                : "bg-[var(--color-muted)]",
            )}
          >
            {g.label}
          </button>
        ))}
      </div>

      {mode === "search" && (
        <form
          className="flex gap-2 md:max-w-md"
          onSubmit={(e) => {
            e.preventDefault();
            void loadSearch();
          }}
        >
          <div className="relative min-w-0 flex-1">
            <Input
              className="h-11 pr-9 text-base md:h-9 md:text-sm"
              placeholder="Search model name…"
              value={query}
              enterKeyHint="search"
              onChange={(e) => setQuery(e.target.value)}
            />
            {query && (
              <button
                type="button"
                aria-label="Clear"
                className="absolute right-1 top-1/2 -translate-y-1/2 rounded p-2"
                onClick={() => setQuery("")}
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>
          <Button type="submit" disabled={loading || query.trim().length < 3}>
            <Search className="h-4 w-4 md:mr-1.5" />
            <span className="hidden md:inline">Search</span>
          </Button>
        </form>
      )}

      {mode === "face" && (
        <label className="inline-flex cursor-pointer items-center gap-2 text-sm">
          <Button type="button" asChild disabled={loading}>
            <span>
              <ScanFace className="mr-1.5 inline h-4 w-4" />
              Upload face image
            </span>
          </Button>
          <input
            type="file"
            accept="image/*"
            className="sr-only"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void f.arrayBuffer().then((buf) => runFaceSearch(buf));
            }}
          />
        </label>
      )}

      {mode === "random" && (
        <Button
          type="button"
          variant="secondary"
          onClick={() => void loadRandom()}
          disabled={loading}
        >
          <Shuffle className="mr-1.5 h-4 w-4" />
          Shuffle again
        </Button>
      )}

      {error && (
        <ErrorState
          message={error}
          onRetry={() => {
            if (mode === "search") void loadSearch();
            else if (mode === "random") void loadRandom();
          }}
        />
      )}

      <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 md:gap-3 lg:grid-cols-5 xl:grid-cols-6">
        {items.map((item) => {
          const plat = platformFromItem(item);
          const liveSite = liveSiteForPlatform(plat);
          const slug = item.channel ?? item.performers[0] ?? item.title;
          if (liveSite) {
            return (
              <Link
                key={item.id}
                to="/live/$site/$slug"
                params={{ site: liveSite, slug }}
                search={{ fromSearch: "0", searchQuery: "", roomUrl: item.url }}
              >
                <SceneCard item={item} onInfo={(i) => setInfoItem(i)} compactActions={isPhone} />
              </Link>
            );
          }
          return (
            <Link
              key={item.id}
              to="/explore/profile/$platform/$model"
              params={{ platform: plat, model: slug }}
            >
              <SceneCard item={item} onInfo={(i) => setInfoItem(i)} compactActions={isPhone} />
            </Link>
          );
        })}
      </div>

      {loading && !error && <SkeletonGrid count={12} cols={isPhone ? 2 : 4} />}

      {!loading && items.length === 0 && !error && (
        <EmptyState
          icon={<Compass className="h-8 w-8" />}
          title="No accounts yet"
          description={
            mode === "search"
              ? "Search a model name (3+ characters)."
              : mode === "face"
                ? "Upload a face image to find matches."
                : "Tap Shuffle to load random accounts."
          }
        />
      )}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
