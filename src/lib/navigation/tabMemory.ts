/**
 * Per-tab navigation memory for the app shell: switching to Downloads and
 * back returns to the search / listing that was open in Browse (same URL,
 * so the browse results cache restores its loaded pages) at the same
 * scroll position. Session-only; nothing is persisted.
 */

const lastHref = new Map<string, string>();
const scrollByHref = new Map<string, number>();

/** The tab a path belongs to (longest matching root; "/" only for "/"). */
export function tabRootFor(pathname: string, roots: readonly string[]): string | undefined {
  let best: string | undefined;
  for (const root of roots) {
    const match =
      root === "/" ? pathname === "/" : pathname === root || pathname.startsWith(`${root}/`);
    if (match && (!best || root.length > best.length)) best = root;
  }
  return best;
}

export function rememberLocation(roots: readonly string[], pathname: string, href: string): void {
  const root = tabRootFor(pathname, roots);
  if (root && root !== "/") lastHref.set(root, href);
}

export function saveScroll(href: string, y: number): void {
  scrollByHref.set(href, Math.max(0, Math.round(y)));
}

export function savedScroll(href: string): number {
  return scrollByHref.get(href) ?? 0;
}

/** What tapping tab `root` should do from `pathname`. */
export type TabAction =
  { kind: "navigate"; href: string; restoreScroll: boolean } | { kind: "scrollTop" };

export function tabAction(root: string, pathname: string, roots: readonly string[]): TabAction {
  const current = tabRootFor(pathname, roots);
  if (current === root) {
    // Already in this tab: a tap goes back to the tab's start page, and a
    // tap on the start page scrolls to the top.
    return pathname === root
      ? { kind: "scrollTop" }
      : { kind: "navigate", href: root, restoreScroll: false };
  }
  const remembered = lastHref.get(root);
  return remembered
    ? { kind: "navigate", href: remembered, restoreScroll: true }
    : { kind: "navigate", href: root, restoreScroll: false };
}

/**
 * Scroll to `y` once the page is tall enough (results render after the
 * route mounts), giving up after `timeoutMs`.
 */
export function restoreScrollTo(y: number, timeoutMs = 2000): () => void {
  if (y <= 0) return () => {};
  const started = performance.now();
  let frame = 0;
  const tick = () => {
    const maxY = document.documentElement.scrollHeight - window.innerHeight;
    if (maxY >= y || performance.now() - started > timeoutMs) {
      window.scrollTo({ top: Math.min(y, Math.max(0, maxY)) });
      return;
    }
    frame = requestAnimationFrame(tick);
  };
  frame = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(frame);
}

/** Test helper. */
export function resetTabMemory(): void {
  lastHref.clear();
  scrollByHref.clear();
}
