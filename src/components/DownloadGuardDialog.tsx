import { AlertTriangle, WifiOff, HardDrive } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { formatBytes, type GuardBlock } from "@/lib/downloads/guards";

interface DownloadGuardDialogProps {
  block: GuardBlock;
  actionLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Confirmation dialog shown before downloads start on a metered connection
 * or when free space is low. Big touch targets, plain language, and the
 * exact free-space figure so users on small phones can decide fast.
 */
export function DownloadGuardDialog({
  block,
  actionLabel,
  onConfirm,
  onCancel,
}: DownloadGuardDialogProps) {
  if (!block) return null;
  const isCellular = block.kind === "cellular";
  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="max-w-[92vw] sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {isCellular ? (
              <WifiOff className="h-5 w-5 text-amber-400" />
            ) : block.critical ? (
              <AlertTriangle className="h-5 w-5 text-red-400" />
            ) : (
              <HardDrive className="h-5 w-5 text-amber-400" />
            )}
            {isCellular
              ? "Download over cellular?"
              : block.critical
                ? "Storage almost full"
                : "Storage running low"}
          </DialogTitle>
          <DialogDescription>
            {isCellular
              ? "You're on a metered connection. Downloads can use a lot of mobile data."
              : block.critical
                ? `Only ${formatBytes(block.freeBytes)} left on this device. Downloads may fail part-way. Free up space or continue anyway.`
                : `Only ${formatBytes(block.freeBytes)} left on this device. Large downloads may not fit.`}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter className="flex-col gap-2 sm:flex-row">
          <Button
            type="button"
            variant="outline"
            className="min-h-11 w-full sm:w-auto"
            onClick={onCancel}
          >
            Cancel
          </Button>
          <Button
            type="button"
            variant={block.kind === "storage" && block.critical ? "destructive" : "default"}
            className="min-h-11 w-full sm:w-auto"
            onClick={onConfirm}
          >
            {actionLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
