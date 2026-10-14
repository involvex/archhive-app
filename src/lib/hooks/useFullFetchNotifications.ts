import { useEffect, useRef } from "react";
import { toast } from "react-hot-toast";
import { api } from "@/lib/api/client";
import type { OnlineCrawl } from "@/lib/types";

const POLL_MS = 10_000;

/**
 * App-wide popups for full fetches of saved searches: paused by a rate
 * limit, finished, or failed. Polls only while one is running or paused.
 */
export function useFullFetchNotifications() {
  const last = useRef(new Map<string, string>());
  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    const names = new Map<string, string>();
    const tick = async () => {
      let crawls: OnlineCrawl[];
      try {
        crawls = await api.listFullFetches();
      } catch {
        // Older backend / remote host without the route: just stop polling.
        return;
      }
      if (cancelled) return;
      const active = crawls.some((c) => c.state === "running" || c.state === "paused");
      if (crawls.some((c) => !names.has(c.search_id))) {
        const saved = await api.listSavedSearches().catch(() => []);
        saved.forEach((s) => names.set(s.id, s.name));
      }
      for (const c of crawls) {
        const before = last.current.get(c.search_id);
        last.current.set(c.search_id, c.state);
        if (!before || before === c.state) continue;
        const name = names.get(c.search_id) ?? "Saved search";
        if (c.state === "paused") {
          const at = c.resume_at ? new Date(c.resume_at).toLocaleTimeString() : "later";
          toast(`${name}: rate limited, full fetch paused — retrying at ${at}`, {
            duration: 8000,
          });
        } else if (c.state === "done") {
          toast.success(
            `${name}: full fetch done — ${c.pages_done} pages, ${c.new_items} new videos`,
            { duration: 8000 },
          );
        } else if (c.state === "error") {
          toast.error(`${name}: full fetch stopped — ${c.message ?? "error"}`, {
            duration: 10000,
          });
        }
      }
      timer = window.setTimeout(() => void tick(), active ? POLL_MS : POLL_MS * 6);
    };
    void tick();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, []);
}
