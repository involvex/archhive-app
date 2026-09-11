import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { CHANGELOG, type ChangelogEntry, getLatestVersion } from "@/lib/changelog";
import { ChevronDown, ChevronRight, ExternalLink } from "lucide-react";

interface ChangelogDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function ChangelogSection({ entry }: { entry: ChangelogEntry }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="border-b border-[var(--color-border)] pb-4">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center gap-2 text-left"
      >
        <span className="font-mono text-xs font-medium text-[var(--color-primary)]">
          v{entry.version}
        </span>
        <time className="text-sm text-[var(--color-muted-foreground)]">{entry.date}</time>
        {expanded ? (
          <ChevronDown className="ml-auto h-4 w-4 text-[var(--color-muted-foreground)]" />
        ) : (
          <ChevronRight className="ml-auto h-4 w-4 text-[var(--color-muted-foreground)]" />
        )}
      </button>
      {expanded && (
        <ul className="mt-2 space-y-1.5 text-sm">
          {entry.changes.map((c) => (
            <li key={`${c.type}-${c.text}`} className="flex items-start gap-2">
              <span
                className={
                  c.type === "added"
                    ? "text-green-400"
                    : c.type === "fixed"
                      ? "text-blue-400"
                      : "text-amber-400"
                }
              >
                {c.type === "added" ? "+" : c.type === "fixed" ? "✓" : "~"}
              </span>
              <span>{c.text}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function ChangelogDialog({ open, onOpenChange }: ChangelogDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl p-6">
        <DialogHeader>
          <DialogTitle>Changelog</DialogTitle>
        </DialogHeader>
        <div className="max-h-80 overflow-y-auto">
          <div className="space-y-1 pr-4">
            {CHANGELOG.map((entry) => (
              <ChangelogSection key={entry.version} entry={entry} />
            ))}
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-4">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              window.open("https://github.com/archhive-app/archhive-app/releases", "_blank");
            }}
          >
            GitHub Releases
            <ExternalLink className="ml-1 h-3 w-3" />
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function useChangelogDialog(appVersion: string) {
  const [open, setOpen] = useState(false);
  const [dialogKey, setDialogKey] = useState(0);

  const latestVersion = getLatestVersion();

  if (!open && appVersion !== "…" && appVersion !== "dev" && appVersion !== latestVersion) {
    const seen = localStorage.getItem("archhive:changelog:lastVersion");
    if (seen !== appVersion) {
      setTimeout(() => {
        localStorage.setItem("archhive:changelog:lastVersion", appVersion);
        setOpen(true);
        setDialogKey((k) => k + 1);
      }, 500);
    }
  }

  return { open, setOpen, dialogKey, latestVersion };
}
