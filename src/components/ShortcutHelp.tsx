import { useCallback, useEffect, useMemo, useState } from "react";
import { getAllShortcuts } from "@/lib/shortcuts/registry";
import { ShortcutBadge } from "@/components/ui/shortcut-badge";
import { X } from "lucide-react";

export function ShortcutHelp() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    function onOpen() {
      setOpen(true);
    }
    window.addEventListener("shortcut:help", onOpen);
    return () => window.removeEventListener("shortcut:help", onOpen);
  }, []);

  const groups = useMemo(() => {
    const all = getAllShortcuts();
    const map: Record<string, typeof all> = {};
    for (const s of all) {
      const cat = s.category;
      if (!map[cat]) map[cat] = [];
      map[cat].push(s);
    }
    return map;
  }, []);

  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    function handler(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, close]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" onClick={close}>
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm" />
      <div
        className="relative z-50 w-full max-w-lg overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-3">
          <h2 className="text-sm font-semibold">Keyboard Shortcuts</h2>
          <button
            type="button"
            onClick={close}
            className="rounded p-1 hover:bg-[var(--color-accent)]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="max-h-80 overflow-y-auto p-4 space-y-4">
          {(["navigation", "actions", "view"] as const).map((cat) => {
            const items = groups[cat];
            if (!items?.length) return null;
            return (
              <div key={cat}>
                <h3 className="mb-2 text-xs font-medium uppercase tracking-wider text-[var(--color-muted-foreground)]">
                  {cat}
                </h3>
                <div className="space-y-1">
                  {items.map((s) => (
                    <div
                      key={s.id}
                      className="flex items-center justify-between rounded-md px-2 py-1.5 text-sm"
                    >
                      <span>{s.label}</span>
                      <ShortcutBadge keys={s.keys} />
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
