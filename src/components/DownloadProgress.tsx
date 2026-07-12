import type { DownloadJob } from "@/lib/types";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { Pause, Play, Trash2, XCircle } from "lucide-react";

interface DownloadProgressRowProps {
  job: DownloadJob;
  onPause?: (id: string) => void;
  onResume?: (id: string) => void;
  onCancel?: (id: string) => void;
  onDelete?: (id: string) => void;
}

const statusColors: Record<DownloadJob["status"], string> = {
  pending: "text-yellow-400",
  active: "text-blue-400",
  paused: "text-orange-400",
  completed: "text-green-400",
  failed: "text-red-400",
  cancelled: "text-[var(--color-muted-foreground)]",
};

export function DownloadProgressRow({
  job,
  onPause,
  onResume,
  onCancel,
  onDelete,
}: DownloadProgressRowProps) {
  const showProgress = job.status === "active" || job.status === "pending";

  return (
    <div className="space-y-2 rounded-lg border border-[var(--color-border)] p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{job.title || job.url}</p>
          <p className={`text-xs capitalize ${statusColors[job.status]}`}>{job.status}</p>
        </div>
        <div className="flex shrink-0 gap-1">
          {job.status === "paused" && onResume && (
            <Button variant="ghost" size="icon" title="Resume" onClick={() => onResume(job.id)}>
              <Play className="h-4 w-4" />
            </Button>
          )}
          {(job.status === "pending" || job.status === "active") && onPause && (
            <Button variant="ghost" size="icon" title="Pause" onClick={() => onPause(job.id)}>
              <Pause className="h-4 w-4" />
            </Button>
          )}
          {(job.status === "pending" || job.status === "active" || job.status === "paused") &&
            onCancel && (
              <Button variant="ghost" size="icon" title="Cancel" onClick={() => onCancel(job.id)}>
                <XCircle className="h-4 w-4" />
              </Button>
            )}
          {onDelete && (
            <Button variant="ghost" size="icon" title="Delete" onClick={() => onDelete(job.id)}>
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>
      {showProgress && <Progress value={job.progress} />}
      {job.error && <p className="text-xs text-red-400">{job.error}</p>}
    </div>
  );
}
