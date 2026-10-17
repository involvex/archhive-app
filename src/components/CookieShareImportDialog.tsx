import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { guessCookieSiteFromText, importCookiesFromJson } from "@/lib/cookies/import";
import { useShareStore } from "@/lib/stores/share";
import type { SiteInfo } from "@/lib/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

/**
 * Offer-to-import dialog for cookie JSON shared into ArcHive.
 *
 * Typing the site every time is the slow part of importing, so when a shared payload
 * looks like a browser cookie export (see `looksLikeCookieExport`) we skip the
 * Bulk-import flow, stash it in the share store, and ask for one confirmation here.
 * The site is pre-selected from the cookie domains when possible.
 */
export function CookieShareImportDialog() {
  const pending = useShareStore((s) => s.pendingCookieImport);
  const clear = useShareStore((s) => s.clearPendingCookieImport);
  const [sites, setSites] = useState<SiteInfo[]>([]);

  useEffect(() => {
    if (!pending) return;
    let cancelled = false;
    // setState happens in the promise callback, not the effect body.
    void api
      .listSites()
      .then((all) => {
        if (cancelled) return;
        const cookieSites = all.filter((s) => s.requires_cookies);
        setSites(cookieSites.length > 0 ? cookieSites : all);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [pending]);

  return (
    <Dialog
      open={pending !== null}
      onOpenChange={(next) => {
        if (!next) clear();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Import shared cookies?</DialogTitle>
          <DialogDescription>
            The shared text looks like a browser cookie export. Choose the site it belongs to and
            ArcHive stores it in the encrypted vault.
          </DialogDescription>
        </DialogHeader>
        {/* Remount per payload so the form state resets without effect-driven setState. */}
        {pending && (
          <CookieImportForm key={pending} pending={pending} sites={sites} onClose={clear} />
        )}
      </DialogContent>
    </Dialog>
  );
}

function CookieImportForm({
  pending,
  sites,
  onClose,
}: {
  pending: string;
  sites: SiteInfo[];
  onClose: () => void;
}) {
  const [siteId, setSiteId] = useState(() => guessCookieSiteFromText(pending) ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  async function confirmImport() {
    if (!siteId) {
      setError("Pick the site these cookies belong to.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const netscape = importCookiesFromJson(pending, siteId);
      const count = netscape.split("\n").filter((l) => l && !l.startsWith("#")).length;
      await api.saveSiteCookies(siteId, netscape);
      setDone(`Imported ${count} cookies for ${siteId}.`);
      setBusy(false);
      // Let the confirmation paint, then close the offer.
      window.setTimeout(() => onClose(), 900);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Import failed.");
      setBusy(false);
    }
  }

  return (
    <>
      <select
        className="w-full rounded-md border border-[var(--color-border)] bg-transparent px-3 py-2 text-sm"
        value={siteId}
        onChange={(e) => setSiteId(e.target.value)}
      >
        <option value="">Select site…</option>
        {sites.map((s) => (
          <option key={s.id} value={s.id}>
            {s.display_name}
          </option>
        ))}
      </select>

      {error && <p className="text-xs text-red-400">{error}</p>}
      {done && <p className="text-xs text-[var(--color-muted-foreground)]">{done}</p>}

      <DialogFooter>
        <Button variant="outline" onClick={() => onClose()} disabled={busy}>
          Discard
        </Button>
        <Button onClick={() => void confirmImport()} disabled={busy || !siteId}>
          {busy ? "Importing…" : "Import cookies"}
        </Button>
      </DialogFooter>
    </>
  );
}
