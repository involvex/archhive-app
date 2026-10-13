import { useEffect } from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

interface BottomSheetProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  /** Pinned under the scrolling body (e.g. an "Apply" / "Done" button). */
  footer?: React.ReactNode;
  className?: string;
}

/**
 * Bottom sheet on phones, centred dialog from `md` up. The body scrolls inside
 * the sheet (max 85% of the dynamic viewport) so long forms never push their
 * controls under the Android navigation bar.
 */
export function BottomSheet({
  open,
  onClose,
  title,
  children,
  footer,
  className,
}: BottomSheetProps) {
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50" role="dialog" aria-modal="true" aria-label={title}>
      <button
        type="button"
        aria-label={`Close ${title}`}
        className="fixed inset-0 cursor-default bg-black/60"
        onClick={onClose}
      />
      <div
        className={cn(
          "fixed right-0 bottom-0 left-0 flex max-h-[85dvh] flex-col overflow-hidden rounded-t-2xl border-t border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl md:top-1/2 md:right-auto md:bottom-auto md:left-1/2 md:w-[min(42rem,90vw)] md:-translate-x-1/2 md:-translate-y-1/2 md:rounded-2xl md:border",
          className,
        )}
      >
        <div className="mx-auto mt-2 h-1 w-10 rounded-full bg-[var(--color-muted-foreground)] opacity-40 md:hidden" />
        <div className="flex items-center gap-2 px-4 pt-3 pb-2">
          <p className="flex-1 text-sm font-semibold">{title}</p>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="rounded-full p-1.5 hover:bg-[var(--color-muted)]"
          >
            <X className="h-5 w-5" />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto overscroll-contain px-4 pb-4">{children}</div>
        {footer && (
          <div className="border-t border-[var(--color-border)] px-4 pt-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
