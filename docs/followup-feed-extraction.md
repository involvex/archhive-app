# Follow-up: Extract shared Feed/Browse hooks (M1)

Source: code review of Feed tab + player layout changes (Oct 2026).

## Problem

`src/routes/feed/index.tsx` (~700 lines) duplicates ~200 lines from
`src/routes/browse/index.tsx`: saved-search state + all five handlers
(`handleCheck` / `closeNewMatches` / `queueAllNewMatches` / `handlePollAll` /
`handleToggleAutoQueue`), `MAX_NEW_MATCHES = 25`, the new-matches card,
the pull-to-refresh block, and the `subscribeDownloadProgress` active-downloads
strip. The copies have already drifted (`handlePlayRecent` in Feed skips the
deleted-scene guard that `src/routes/index.tsx` has; `refreshSaved` has an extra
`savedLoading` state). The next bug fix in one copy will not reach the other.

## Proposed extraction

- `src/lib/hooks/useSavedSearches.ts` — `saved`, `savedRef`, `checkingId`,
  `queueing`, `pollStatus`, `newMatches`, `MAX_NEW_MATCHES`, and all five
  handlers. Both routes consume it; behavior identical.
- `src/lib/hooks/useTrending.ts` — `(enabledSiteIds) => { trending,
trendingLoading, loadTrending }` wrapping the `api.browse(site, "search",
"trending", 1)` fan-out (top 10 per site). Add concurrency cap + request-id
  guard here (see review P1) once, benefiting both pages.
- `src/components/NewMatchesCard.tsx` — the bordered card rendering
  `newMatches` with Queue-all / Dismiss, shared by both routes.
- Optional: `<FeedSection>` / shared pull-to-refresh + download-subscription
  wrapper if a third consumer appears.

## Acceptance

- `feed/index.tsx` and `browse/index.tsx` import the hooks/component; no
  duplicated handler logic remains (grep `queueAllNewMatches` → one definition).
- `bun run lint`, `bun run format:check`, `bun run build` green.
- Manual: saved-search check/queue/dismiss + trending chips work on both pages.

## Deferred companions (from the same review)

All items completed:

- **P1** ✅ Expanded in `useTrending.ts` (max 3 concurrent workers, stale-response
  guard via `requestIdRef`) and `useRefreshGuard.ts` (in-flight protection).
- **S3** ✅ Implemented in `NewMatchesCard.tsx` (count + total confirm dialog) and
  `api/client.ts` (safeMediaUrl for https-only guard on stream URLs).
- **S4** ✅ Hardened `path_slug` in `src-tauri/src/sites/urls.rs` to
  `[a-z0-9-_]` (underscore kept per codebase convention) with host-assert
  `www.pornhub.com` in `assert_pornhub_url`, plus `&`/unicode tests in
  `pornhub.rs::test`.
