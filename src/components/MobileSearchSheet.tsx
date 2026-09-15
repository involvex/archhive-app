import { useEffect, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "react-hot-toast";
import {
  Compass,
  Copy,
  Download,
  Home,
  Library,
  Plus,
  Radio,
  RefreshCw,
  Search,
  Settings,
  User,
  X,
} from "lucide-react";
import { api } from "@/lib/api/client";
import type { Performer, Scene } from "@/lib/types";

interface MobileSearchSheetProps {
  onClose: () => void;
}

interface GoItem {
  id: string;
  label: string;
  icon: React.ReactNode;
  run: () => void;
}

const RESULT_LIMIT_SCENES = 6;
const RESULT_LIMIT_PERFORMERS = 4;
const DEBOUNCE_MS = 250;

/**
 * Touch-first replacement for the desktop command palette (Q32/#50).
 * Quick navigation + actions when idle; live library results while typing.
 * Scene/performer taps hand the query to the list pages via the `q` search
 * param, which they consume into their local filter boxes.
 */
export function MobileSearchSheet({ onClose }: MobileSearchSheetProps) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [performers, setPerformers] = useState<Performer[]>([]);
  const [searching, setSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 50);
  }, []);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setScenes([]);
      setPerformers([]);
      setSearching(false);
      return;
    }
    setSearching(true);
    let cancelled = false;
    const id = window.setTimeout(() => {
      void Promise.all([
        api.listScenes(q, "newest").catch(() => [] as Scene[]),
        api.listPerformers(q).catch(() => [] as Performer[]),
      ]).then(([s, p]) => {
        if (cancelled) return;
        setScenes(s.slice(0, RESULT_LIMIT_SCENES));
        setPerformers(p.slice(0, RESULT_LIMIT_PERFORMERS));
        setSearching(false);
      });
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(id);
    };
  }, [query]);

  function go(to: string) {
    onClose();
    setTimeout(() => navigate({ to }), 10);
  }

  async function scanLibrary() {
    onClose();
    try {
      const r = await api.scanLibrary();
      toast.success(`Scan done: ${r.added} added, ${r.updated} updated`);
    } catch {
      toast.error("Library scan failed");
    }
  }

  const goItems: GoItem[] = [
    { id: "go-home", label: "Home", icon: <Home className="h-4 w-4" />, run: () => go("/") },
    {
      id: "go-browse",
      label: "Browse",
      icon: <Compass className="h-4 w-4" />,
      run: () => go("/browse"),
    },
    {
      id: "go-library",
      label: "Library",
      icon: <Library className="h-4 w-4" />,
      run: () => go("/library"),
    },
    { id: "go-live", label: "Live", icon: <Radio className="h-4 w-4" />, run: () => go("/live") },
    {
      id: "go-downloads",
      label: "Downloads",
      icon: <Download className="h-4 w-4" />,
      run: () => go("/downloads"),
    },
    {
      id: "go-duplicates",
      label: "Duplicates",
      icon: <Copy className="h-4 w-4" />,
      run: () => go("/duplicates"),
    },
    {
      id: "go-settings",
      label: "Settings",
      icon: <Settings className="h-4 w-4" />,
      run: () => go("/settings"),
    },
    {
      id: "action-new-download",
      label: "New download",
      icon: <Plus className="h-4 w-4" />,
      run: () => go("/browse/by-url"),
    },
    {
      id: "action-scan-library",
      label: "Scan library",
      icon: <RefreshCw className="h-4 w-4" />,
      run: () => void scanLibrary(),
    },
  ];

  const q = query.trim();
  const showResults = q.length > 0;
  const emptyResults = showResults && !searching && scenes.length === 0 && performers.length === 0;

  function openScenesWithQuery() {
    onClose();
    setTimeout(() => navigate({ to: "/library/scenes", search: { q } }), 10);
  }

  function openPerformersWithQuery() {
    onClose();
    setTimeout(() => navigate({ to: "/library/performers", search: { q } }), 10);
  }

  return (
    <div
      className="fixed inset-0 z-50 md:hidden"
      role="dialog"
      aria-modal="true"
      aria-label="Search"
    >
      <button
        type="button"
        aria-label="Close search"
        className="fixed inset-0 cursor-default bg-black/60"
        onClick={onClose}
      />
      <div className="fixed right-0 bottom-0 left-0 max-h-[82dvh] overflow-hidden rounded-t-2xl border-t border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl">
        <div className="mx-auto mt-2 h-1 w-10 rounded-full bg-[var(--color-muted-foreground)] opacity-40" />
        <div className="flex items-center gap-2 border-b border-[var(--color-border)] px-4 py-3">
          <Search className="h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search scenes, performers, or jump anywhere…"
            enterKeyHint="search"
            className="min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-[var(--color-muted-foreground)]"
          />
          {searching && <span className="text-xs text-[var(--color-muted-foreground)]">…</span>}
          <button
            type="button"
            onClick={onClose}
            aria-label="Close search"
            className="rounded p-2 hover:bg-[var(--color-muted)]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="max-h-[60dvh] overflow-y-auto overscroll-contain p-2 pb-[env(safe-area-inset-bottom)]">
          {!showResults && (
            <div>
              <p className="px-2 pt-1 pb-1 text-xs font-medium tracking-wider text-[var(--color-muted-foreground)] uppercase">
                Go to
              </p>
              {goItems.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={item.run}
                  className="flex min-h-12 w-full items-center gap-3 rounded-md px-3 py-2.5 text-left text-sm hover:bg-[var(--color-muted)]"
                >
                  <span className="text-[var(--color-muted-foreground)]">{item.icon}</span>
                  {item.label}
                </button>
              ))}
            </div>
          )}
          {showResults && scenes.length > 0 && (
            <div>
              <p className="px-2 pt-1 pb-1 text-xs font-medium tracking-wider text-[var(--color-muted-foreground)] uppercase">
                Scenes
              </p>
              {scenes.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  onClick={openScenesWithQuery}
                  className="flex min-h-12 w-full items-center gap-3 rounded-md px-3 py-2.5 text-left hover:bg-[var(--color-muted)]"
                >
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">{s.title}</span>
                    {s.performers.length > 0 && (
                      <span className="block truncate text-xs text-[var(--color-muted-foreground)]">
                        {s.performers.join(", ")}
                      </span>
                    )}
                  </span>
                </button>
              ))}
            </div>
          )}
          {showResults && performers.length > 0 && (
            <div>
              <p className="px-2 pt-1 pb-1 text-xs font-medium tracking-wider text-[var(--color-muted-foreground)] uppercase">
                Performers
              </p>
              {performers.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  onClick={openPerformersWithQuery}
                  className="flex min-h-12 w-full items-center gap-3 rounded-md px-3 py-2.5 text-left hover:bg-[var(--color-muted)]"
                >
                  <User className="h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]" />
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">{p.name}</span>
                  <span className="shrink-0 text-xs text-[var(--color-muted-foreground)]">
                    {p.scene_count}
                  </span>
                </button>
              ))}
            </div>
          )}
          {emptyResults && (
            <p className="px-3 py-6 text-center text-sm text-[var(--color-muted-foreground)]">
              No matches for “{q}”.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
