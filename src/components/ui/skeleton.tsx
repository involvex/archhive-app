import { cn } from "@/lib/utils";

function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("bg-[var(--color-muted)]/60 animate-pulse rounded-md", className)}
      {...props}
    />
  );
}

export { Skeleton };
