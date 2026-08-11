import { cn } from "@/lib/utils";

interface ShortcutBadgeProps {
  keys: string;
  className?: string;
}

export function ShortcutBadge({ keys, className }: ShortcutBadgeProps) {
  const parts = keys.split("+").map((k) => k.trim());
  return (
    <span className={cn("inline-flex items-center gap-0.5", className)}>
      {parts.map((part) => {
        const label =
          part === "ctrl"
            ? "Ctrl"
            : part === "shift"
              ? "Shift"
              : part === "alt"
                ? "Alt"
                : part === "meta"
                  ? "Cmd"
                  : part.length === 1
                    ? part.toUpperCase()
                    : part.charAt(0).toUpperCase() + part.slice(1);
        return (
          <kbd
            key={`kbd-${label}`}
            className="inline-flex h-5 min-w-[20px] items-center justify-center rounded border border-[var(--color-border)] bg-[var(--color-muted)] px-1 font-mono text-[10px] text-[var(--color-muted-foreground)]"
          >
            {label}
          </kbd>
        );
      })}
    </span>
  );
}
