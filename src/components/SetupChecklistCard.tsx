import { Link } from "@tanstack/react-router";
import { CheckCircle2, Circle, ListChecks, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { SetupChecklistItem } from "@/lib/setupChecklist";

export interface SetupChecklistCardProps {
  items: SetupChecklistItem[];
  onDismiss: () => void;
}

export function SetupChecklistCard({ items, onDismiss }: SetupChecklistCardProps) {
  const doneCount = items.filter((i) => i.done).length;

  return (
    <Card className="border-[var(--color-primary)]/40">
      <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0 pb-2">
        <div className="space-y-1">
          <CardTitle className="flex items-center gap-2 text-base">
            <ListChecks className="h-4 w-4 text-[var(--color-primary)]" />
            Finish setup
          </CardTitle>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            {doneCount}/{items.length} complete — open each item to adjust Settings
          </p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-9 w-9 shrink-0"
          onClick={onDismiss}
          aria-label="Dismiss setup checklist"
        >
          <X className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent className="space-y-2">
        <ul className="space-y-2">
          {items.map((item) => (
            <li key={item.id}>
              <Link
                to="/settings"
                search={{ tab: item.tab }}
                className="flex min-h-11 items-start gap-3 rounded-md border border-[var(--color-border)] px-3 py-2 text-left transition-colors hover:bg-[var(--color-muted)]"
              >
                {item.done ? (
                  <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-green-500" aria-hidden />
                ) : (
                  <Circle
                    className="mt-0.5 h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]"
                    aria-hidden
                  />
                )}
                <span className="min-w-0 flex-1">
                  <span className="block text-sm font-medium">{item.title}</span>
                  <span className="block text-xs text-[var(--color-muted-foreground)]">
                    {item.description}
                  </span>
                </span>
                <span className="shrink-0 text-xs text-[var(--color-primary)]">
                  {item.done ? "Review" : "Open"}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
