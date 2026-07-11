import type { MediaItem } from "@/lib/types";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Download, Clock } from "lucide-react";

interface SceneCardProps {
  item: MediaItem;
  onDownload?: (item: MediaItem) => void;
}

export function SceneCard({ item, onDownload }: SceneCardProps) {
  return (
    <Card className="overflow-hidden transition hover:border-[var(--color-primary)]">
      <div className="aspect-video bg-[var(--color-muted)] relative">
        {item.thumbnail ? (
          <img src={item.thumbnail} alt={item.title} className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full items-center justify-center text-[var(--color-muted-foreground)] text-xs">
            No preview
          </div>
        )}
        {item.duration ? (
          <span className="absolute bottom-2 right-2 rounded bg-black/70 px-1.5 py-0.5 text-xs flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDuration(item.duration)}
          </span>
        ) : null}
      </div>
      <CardContent className="p-3 space-y-2">
        <p className="line-clamp-2 text-sm font-medium leading-tight">{item.title}</p>
        {item.performers.length > 0 && (
          <p className="text-xs text-[var(--color-muted-foreground)] truncate">
            {item.performers.join(", ")}
          </p>
        )}
        {onDownload && (
          <Button size="sm" className="w-full" onClick={() => onDownload(item)}>
            <Download className="h-3.5 w-3.5" />
            Download
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}
