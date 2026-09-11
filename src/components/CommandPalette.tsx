import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { getAllShortcuts, type Shortcut } from "@/lib/shortcuts/registry";
import { ShortcutBadge } from "@/components/ui/shortcut-badge";
import { api } from "@/lib/api/client";
import {
  Search,
  Home,
  Compass,
  Library,
  Radio,
  Download,
  Copy,
  Settings,
  Folder,
} from "lucide-react";

interface PaletteItem {
  id: string;
  label: string;
  category: string;
  icon?: React.ReactNode;
  shortcut?: string;
  action: () => void;
}

const NAV_LINKS = [
  {
    id: "nav-home",
    label: "Home",
    path: "/",
    icon: <Home className="h-4 w-4" />,
    shortcut: "Ctrl+1",
  },
  {
    id: "nav-browse",
    label: "Browse",
    path: "/browse",
    icon: <Compass className="h-4 w-4" />,
    shortcut: "Ctrl+2",
  },
  {
    id: "nav-library",
    label: "Library",
    path: "/library",
    icon: <Library className="h-4 w-4" />,
    shortcut: "Ctrl+3",
  },
  {
    id: "nav-live",
    label: "Live",
    path: "/live",
    icon: <Radio className="h-4 w-4" />,
    shortcut: "Ctrl+4",
  },
  {
    id: "nav-downloads",
    label: "Downloads",
    path: "/downloads",
    icon: <Download className="h-4 w-4" />,
    shortcut: "Ctrl+5",
  },
  {
    id: "nav-settings",
    label: "Settings",
    path: "/settings",
    icon: <Settings className="h-4 w-4" />,
    shortcut: "Ctrl+6",
  },
  {
    id: "nav-duplicates",
    label: "Duplicates",
    path: "/duplicates",
    icon: <Copy className="h-4 w-4" />,
    shortcut: "Ctrl+7",
  },
];

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  const shortcuts = useMemo(() => getAllShortcuts(), []);

  const navItems: PaletteItem[] = useMemo(
    () =>
      NAV_LINKS.map((link) => ({
        id: link.id,
        label: link.label,
        category: "Navigation",
        icon: link.icon,
        shortcut: link.shortcut,
        action: () => {
          setOpen(false);
          setTimeout(() => navigate({ to: link.path }), 10);
        },
      })),
    [navigate],
  );

  const actionItems: PaletteItem[] = useMemo(
    () => [
      {
        id: "action-new-download",
        label: "New download (Ctrl+N)",
        category: "Actions",
        icon: <Download className="h-4 w-4" />,
        action: () => {
          setOpen(false);
          setTimeout(() => navigate({ to: "/browse/by-url" }), 10);
        },
      },
      {
        id: "action-scan-library",
        label: "Scan library",
        category: "Actions",
        icon: <Folder className="h-4 w-4" />,
        action: () => {
          setOpen(false);
          void api.scanLibrary();
        },
      },
    ],
    [navigate],
  );

  const allItems = useMemo(() => [...navItems, ...actionItems], [navItems, actionItems]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase().trim();
    if (!q) {
      return [...shortcuts, ...allItems];
    }
    return allItems.filter(
      (item) => item.label.toLowerCase().includes(q) || item.category.toLowerCase().includes(q),
    );
  }, [shortcuts, allItems, query]);

  useEffect(() => {
    if (!open) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setSelectedIdx(0);
    setQuery("");
    setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  useEffect(() => {
    function onOpen() {
      setOpen(true);
    }
    window.addEventListener("shortcut:cmd-palette", onOpen);
    return () => window.removeEventListener("shortcut:cmd-palette", onOpen);
  }, []);

  const execute = useCallback((item: PaletteItem) => {
    setOpen(false);
    setTimeout(() => item.action(), 10);
  }, []);

  const executeShortcut = useCallback((s: Shortcut) => {
    setOpen(false);
    setTimeout(() => s.action(), 10);
  }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      setOpen(false);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && filtered[selectedIdx]) {
      const item = filtered[selectedIdx];
      if ("keys" in item) {
        executeShortcut(item as Shortcut);
      } else {
        execute(item as PaletteItem);
      }
    }
  }

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
      onClick={() => setOpen(false)}
    >
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm" />
      <div
        className="relative z-50 w-full max-w-md overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center border-b border-[var(--color-border)] px-3">
          <Search className="h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIdx(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Type a command, search, or route..."
            className="flex-1 bg-transparent py-3 px-2 text-sm outline-none placeholder:text-[var(--color-muted-foreground)]"
          />
        </div>
        <div ref={listRef} className="max-h-72 overflow-y-auto p-1">
          {filtered.length === 0 && (
            <p className="py-6 text-center text-sm text-[var(--color-muted-foreground)]">
              No commands found
            </p>
          )}
          {filtered.map((item, i) => {
            const isShortcut = "keys" in item;
            const icon = (item as PaletteItem).icon;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() =>
                  isShortcut ? executeShortcut(item as Shortcut) : execute(item as PaletteItem)
                }
                onMouseEnter={() => setSelectedIdx(i)}
                className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-sm transition-colors ${
                  i === selectedIdx
                    ? "bg-[var(--color-accent)] text-[var(--color-accent-foreground)]"
                    : "text-[var(--color-foreground)]"
                }`}
              >
                <span className="flex items-center gap-2">
                  {icon}
                  <span className="text-xs text-[var(--color-muted-foreground)]">
                    {(item as PaletteItem).category}
                  </span>
                  {(item as PaletteItem).label}
                </span>
                {isShortcut && <ShortcutBadge keys={(item as Shortcut).keys} />}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
