import type { DownloadJob } from "@/lib/types";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { XCircle } from "lucide-react";

interface DownloadProgressRowProps {
  job: DownloadJob;
  onCancel?: (id: string) => void;
}

const statusColors: Record<DownloadJob["status"], string> = {
  pending: "text-yellow-400",
  active: "text-blue-400",
  completed: "text-green-400",
  failed: "text-red-400",
  cancelled: "text-[var(--color-muted-foreground)]",
};

export function DownloadProgressRow({ job, onCancel }: DownloadProgressRowProps) {
  return (
    <div className="rounded-lg border border-[var(--color-border)] p-3 space-y-2">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{job.title || job.url}</p>
          <p className={`text-xs capitalize ${statusColors[job.status]}`}>{job.status}</p>
        </div>
        {(job.status === "pending" || job.status === "active") && onCancel && (
          <Button variant="ghost" size="icon" onClick={() => onCancel(job.id)}>
            <XCircle className="h-4 w-4" />
          </Button>
        )}
      </div>
      {(job.status === "active" || job.status === "pending") && <Progress value={job.progress} />}
      {job.error && <p className="text-xs text-red-400">{job.error}</p>}
    </div>
  );
}
