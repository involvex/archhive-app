import { useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { classifyBulkUrls } from "@/lib/parseBulkUrls";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ClipboardPaste } from "lucide-react";
import * as Switch from "@radix-ui/react-switch";

interface BulkImportPanelProps {
  onQueued: () => void;
}

export function BulkImportPanel({ onQueued }: BulkImportPanelProps) {
  const [text, setText] = useState("");
  const [status, setStatus] = useState("");
  const [importing, setImporting] = useState(false);
  const [expandBrowse, setExpandBrowse] = useState(true);
  const [importAll, setImportAll] = useState(false);

  const { videos, browse, other } = useMemo(
    () => classifyBulkUrls(text, importAll),
    [text, importAll],
  );

  const totalToQueue = importAll
    ? videos.length + browse.length + other.length
    : videos.length + (expandBrowse ? browse.length : 0);

  async function handlePaste() {
    try {
      const clip = await navigator.clipboard.readText();
      if (clip.trim()) setText(clip);
    } catch {
      setStatus("Could not read clipboard — paste manually into the box.");
    }
  }

  async function queueAll() {
    const urls = parseBulkUrlsFromState();
    if (urls.length === 0) return;
    setImporting(true);
    setStatus(`Importing ${urls.length} URL(s)…`);
    try {
      const result = await api.queueBulkImport(urls, expandBrowse, importAll);
      setStatus(
        `Queued ${result.queued} download(s)` +
          (result.expanded > 0 ? ` from ${result.expanded} channel/search page(s)` : "") +
          (result.skipped > 0 ? ` · ${result.skipped} skipped` : "") +
          ". Max 2 run at once.",
      );
      setText("");
      onQueued();
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Bulk import failed");
    } finally {
      setImporting(false);
    }
  }

  function parseBulkUrlsFromState(): string[] {
    const all = classifyBulkUrls(text, importAll);
    if (importAll) {
      return [...all.videos, ...all.browse, ...all.other];
    }
    return [...all.videos, ...(expandBrowse ? all.browse : [])];
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ClipboardPaste className="h-4 w-4" />
          Bulk import URLs
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-xs text-[var(--color-muted-foreground)]">
          Paste a numbered list from Kiwi or any browser. Watch links queue directly; channel and
          search links expand via yt-dlp (up to 100 videos each).
        </p>
        <textarea
          className="min-h-[120px] w-full rounded-md border border-[var(--color-border)] bg-[var(--color-background)] p-3 font-mono text-sm"
          placeholder={"1. https://www.youporn.com/watch/123/\n2. https://…/channel/…"}
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2 text-sm">
            <Switch.Root
              checked={expandBrowse}
              onCheckedChange={setExpandBrowse}
              className="h-5 w-9 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
            >
              <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
            </Switch.Root>
            Expand channel/search
          </label>
          <label className="flex items-center gap-2 text-sm">
            <Switch.Root
              checked={importAll}
              onCheckedChange={setImportAll}
              className="h-5 w-9 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
            >
              <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
            </Switch.Root>
            Import all URLs
          </label>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => void handlePaste()}>
            Paste from clipboard
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={importing || totalToQueue === 0}
            onClick={() => void queueAll()}
          >
            Import {totalToQueue > 0 ? totalToQueue : ""} URL
            {totalToQueue === 1 ? "" : "s"}
          </Button>
        </div>
        {text.trim() && (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            {videos.length} video{videos.length === 1 ? "" : "s"}
            {browse.length > 0
              ? ` · ${browse.length} channel/search (expand${expandBrowse ? "" : " off"})`
              : ""}
            {other.length > 0 && importAll ? ` · ${other.length} other` : ""}
            {other.length > 0 && !importAll
              ? ` · ${other.length} unrecognized (enable Import all)`
              : ""}
          </p>
        )}
        {browse.length > 0 && expandBrowse && (
          <ul className="max-h-20 overflow-y-auto text-xs text-[var(--color-muted-foreground)]">
            {browse.map((url) => (
              <li key={url} className="truncate">
                Browse expand: {url}
              </li>
            ))}
          </ul>
        )}
        {status && <p className="text-sm">{status}</p>}
      </CardContent>
    </Card>
  );
}
