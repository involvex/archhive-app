import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-[var(--color-border)] px-6 py-12 text-center",
        className,
      )}
    >
      {icon && <div className="text-[var(--color-muted-foreground)]">{icon}</div>}
      <div className="space-y-1">
        <p className="text-sm font-medium">{title}</p>
        {description && (
          <p className="text-xs text-[var(--color-muted-foreground)] max-w-sm">{description}</p>
        )}
      </div>
      {action}
    </div>
  );
}
