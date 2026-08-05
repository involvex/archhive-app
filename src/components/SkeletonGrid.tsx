import { Skeleton } from "@/components/ui/skeleton";

interface SkeletonGridProps {
  count?: number;
  cols?: number;
  aspectClass?: string;
}

export function SkeletonGrid({
  count = 12,
  cols = 3,
  aspectClass = "aspect-video",
}: SkeletonGridProps) {
  return (
    <div
      className="grid gap-3"
      style={{
        gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`,
      }}
    >
      {Array.from({ length: count }, (_, i) => i + 1).map((n) => (
        <div key={n} className="overflow-hidden rounded-lg border border-[var(--color-border)]">
          <Skeleton className={aspectClass} />
          <div className="space-y-2 p-3">
            <Skeleton className="h-4 w-3/4" />
            <Skeleton className="h-3 w-1/2" />
          </div>
        </div>
      ))}
    </div>
  );
}
