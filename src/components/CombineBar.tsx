import { X } from "lucide-react";
import { Button } from "@/components/ui/button";

interface CombineBarProps {
  /** Picked tags / categories, in order. */
  selected: { key: string; label: string }[];
  /** Explains what combining does on this site (exact vs word search). */
  hint: string;
  onRemove: (key: string) => void;
  onBrowse: () => void;
}

/**
 * Floating bar for Combine mode: stays visible above the phone's bottom
 * navigation (and clear of the search FAB) while the user scrolls a long tag
 * list, so the picks and "Browse" never scroll out of reach.
 */
export function CombineBar({ selected, hint, onRemove, onBrowse }: CombineBarProps) {
  return (
    <div className="fixed right-20 bottom-[calc(5.5rem+env(safe-area-inset-bottom))] left-3 z-40 rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)]/95 p-2.5 shadow-2xl backdrop-blur md:right-6 md:bottom-6 md:left-auto md:w-[28rem]">
      {selected.length === 0 ? (
        <p className="px-1 py-1.5 text-xs text-[var(--color-muted-foreground)]">{hint}</p>
      ) : (
        <div className="flex items-center gap-2">
          <div className="flex min-w-0 flex-1 gap-1.5 overflow-x-auto scrollbar-hide">
            {selected.map((s) => (
              <button
                key={s.key}
                type="button"
                onClick={() => onRemove(s.key)}
                className="inline-flex shrink-0 items-center gap-1 rounded-full bg-[var(--color-primary)] px-2.5 py-1 text-xs text-[var(--color-primary-foreground)]"
              >
                {s.label}
                <X className="h-3 w-3" />
              </button>
            ))}
          </div>
          <Button size="sm" className="h-9 shrink-0" onClick={onBrowse}>
            Browse{selected.length > 1 ? ` ${selected.length}` : ""}
          </Button>
        </div>
      )}
    </div>
  );
}
