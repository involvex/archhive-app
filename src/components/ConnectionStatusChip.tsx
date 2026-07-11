import { Link } from "@tanstack/react-router";
import { RefreshCw, Wifi, WifiOff } from "lucide-react";
import { useLanConnection } from "@/hooks/useLanConnection";
import { Button } from "@/components/ui/button";

interface ConnectionStatusChipProps {
  showRefresh?: boolean;
  className?: string;
}

export function ConnectionStatusChip({
  showRefresh = true,
  className = "",
}: ConnectionStatusChipProps) {
  const { state, message, refresh } = useLanConnection();

  const connected = state === "connected";
  const checking = state === "checking";
  const Icon = connected ? Wifi : WifiOff;
  const color = connected
    ? "border-green-600/40 bg-green-950/30 text-green-200"
    : checking
      ? "border-yellow-600/40 bg-yellow-950/30 text-yellow-200"
      : "border-red-600/40 bg-red-950/30 text-red-200";

  return (
    <div
      className={`flex flex-wrap items-center gap-2 rounded-md border px-3 py-2 text-sm ${color} ${className}`}
    >
      <Icon className="h-4 w-4 shrink-0" />
      <span className="min-w-0 flex-1">{message}</span>
      {!connected && state === "error" && (
        <Button asChild variant="outline" size="sm" className="h-7 text-xs">
          <Link to="/settings">Settings</Link>
        </Button>
      )}
      {showRefresh && (
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2"
          onClick={() => void refresh()}
          disabled={checking}
          aria-label="Refresh connection"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${checking ? "animate-spin" : ""}`} />
        </Button>
      )}
    </div>
  );
}
