import { useRef, useState } from "react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export interface Suggestion {
  /** Value written into the input when picked. */
  value: string;
  /** Optional display text (defaults to `value`). */
  label?: string;
  /** Optional right-aligned hint (e.g. a video count). */
  hint?: string;
}

interface SuggestInputProps extends Omit<React.ComponentProps<"input">, "onChange" | "value"> {
  value: string;
  onValueChange: (value: string) => void;
  suggestions: Suggestion[];
  /** Called after a suggestion is picked (e.g. to apply it right away). */
  onPick?: (value: string) => void;
  /** Most rows shown before the list scrolls. */
  maxVisible?: number;
}

/**
 * Text input with an in-page suggestion list. Replaces `<datalist>`, which
 * Android WebView renders as a native popup that can cover the whole screen
 * and even draw over the keyboard. This list is part of the page: short,
 * scrollable, anchored under the input, and closed on pick / blur / Escape.
 */
export function SuggestInput({
  value,
  onValueChange,
  suggestions,
  onPick,
  maxVisible = 6,
  className,
  onFocus,
  onBlur,
  onKeyDown,
  ...props
}: SuggestInputProps) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const shown = open && suggestions.length > 0;

  function pick(s: Suggestion) {
    onValueChange(s.value);
    setOpen(false);
    setActive(-1);
    onPick?.(s.value);
  }

  return (
    <div className="relative min-w-0 flex-1">
      <Input
        {...props}
        ref={inputRef}
        value={value}
        autoComplete="off"
        className={className}
        role="combobox"
        aria-expanded={shown}
        aria-autocomplete="list"
        onChange={(e) => {
          onValueChange(e.target.value);
          setOpen(true);
          setActive(-1);
        }}
        onFocus={(e) => {
          setOpen(true);
          // Keep the input near the top so the list fits above the keyboard.
          setTimeout(
            () => inputRef.current?.scrollIntoView({ block: "start", behavior: "smooth" }),
            250,
          );
          onFocus?.(e);
        }}
        onBlur={(e) => {
          setOpen(false);
          onBlur?.(e);
        }}
        onKeyDown={(e) => {
          if (shown && e.key === "ArrowDown") {
            e.preventDefault();
            setActive((i) => Math.min(i + 1, suggestions.length - 1));
            return;
          }
          if (shown && e.key === "ArrowUp") {
            e.preventDefault();
            setActive((i) => Math.max(i - 1, -1));
            return;
          }
          if (shown && e.key === "Enter" && active >= 0) {
            e.preventDefault();
            pick(suggestions[active]);
            return;
          }
          if (e.key === "Escape") setOpen(false);
          onKeyDown?.(e);
        }}
      />
      {shown && (
        <ul
          role="listbox"
          className="absolute top-full right-0 left-0 z-30 mt-1 overflow-y-auto overscroll-contain rounded-md border border-[var(--color-border)] bg-[var(--color-card)] py-1 shadow-xl"
          // ~2.5rem per row; the list scrolls past `maxVisible` rows.
          style={{ maxHeight: `${maxVisible * 2.5}rem` }}
        >
          {suggestions.map((s, i) => (
            <li key={s.value} role="option" aria-selected={i === active}>
              <button
                type="button"
                // Keep focus in the input so blur doesn't close the list first.
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pick(s)}
                className={cn(
                  "flex h-10 w-full items-center justify-between gap-2 px-3 text-left text-sm",
                  i === active ? "bg-[var(--color-muted)]" : "active:bg-[var(--color-muted)]",
                )}
              >
                <span className="truncate">{s.label ?? s.value}</span>
                {s.hint && (
                  <span className="shrink-0 text-xs text-[var(--color-muted-foreground)]">
                    {s.hint}
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
