import { AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorState({ message, onRetry, className }: ErrorStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-3 rounded-lg border border-red-400/30 bg-red-400/10 px-6 py-8 text-center",
        className,
      )}
    >
      <AlertCircle className="h-8 w-8 text-red-400" />
      <p className="text-sm text-red-300">{message}</p>
      {onRetry && (
        <Button size="sm" variant="outline" onClick={onRetry} className="border-red-400/30">
          Retry
        </Button>
      )}
    </div>
  );
}
