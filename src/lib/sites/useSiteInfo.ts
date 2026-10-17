import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { SiteInfo } from "@/lib/types";
import { SITE_CATALOG } from "./catalog";

let sitesPromise: Promise<SiteInfo[]> | null = null;

/** Backend site list (with browse options), fetched once per session. */
export function loadSites(): Promise<SiteInfo[]> {
  if (!sitesPromise) {
    sitesPromise = api.listSites().catch((e: unknown) => {
      sitesPromise = null;
      throw e;
    });
  }
  return sitesPromise;
}

/**
 * Site metadata for `siteId`: the static catalog entry immediately, replaced
 * by the backend entry (which carries `browse_options`) once it loads.
 */
export function useSiteInfo(siteId: string): SiteInfo | undefined {
  const [site, setSite] = useState<SiteInfo | undefined>(() =>
    SITE_CATALOG.find((s) => s.id === siteId),
  );
  useEffect(() => {
    let cancelled = false;
    loadSites()
      .then((sites) => {
        const hit = sites.find((s) => s.id === siteId);
        if (!cancelled && hit) setSite(hit);
      })
      .catch(() => {
        /* keep the static entry */
      });
    return () => {
      cancelled = true;
    };
  }, [siteId]);
  return site;
}
