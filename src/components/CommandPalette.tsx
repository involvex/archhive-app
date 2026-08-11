import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getAllShortcuts, type Shortcut } from "@/lib/shortcuts/registry";
import { ShortcutBadge } from "@/components/ui/shortcut-badge";
import { Search } from "lucide-react";

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const shortcuts = useMemo(() => getAllShortcuts(), [open]);

  const filtered = useMemo(() => {
    if (!query.trim()) return shortcuts;
    const q = query.toLowerCase();
    return shortcuts.filter(
      (s) =>
        s.label.toLowerCase().includes(q) ||
        s.keys.toLowerCase().includes(q) ||
        s.category.toLowerCase().includes(q),
    );
  }, [shortcuts, query]);

  useEffect(() => {
    if (!open) return;
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

  const execute = useCallback((s: Shortcut) => {
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
      execute(filtered[selectedIdx]);
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
            placeholder="Type a command or search..."
            className="flex-1 bg-transparent py-3 px-2 text-sm outline-none placeholder:text-[var(--color-muted-foreground)]"
          />
        </div>
        <div ref={listRef} className="max-h-72 overflow-y-auto p-1">
          {filtered.length === 0 && (
            <p className="py-6 text-center text-sm text-[var(--color-muted-foreground)]">
              No commands found
            </p>
          )}
          {filtered.map((s, i) => (
            <button
              key={s.id}
              type="button"
              onClick={() => execute(s)}
              onMouseEnter={() => setSelectedIdx(i)}
              className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-sm transition-colors ${
                i === selectedIdx
                  ? "bg-[var(--color-accent)] text-[var(--color-accent-foreground)]"
                  : "text-[var(--color-foreground)]"
              }`}
            >
              <span className="flex items-center gap-2">
                <span className="text-xs text-[var(--color-muted-foreground)]">{s.category}</span>
                {s.label}
              </span>
              <ShortcutBadge keys={s.keys} />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
