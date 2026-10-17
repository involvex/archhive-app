# Combining tags, categories and searches

What each site can **strictly** combine in one listing, i.e. results that must
match _every_ term, versus what only works as a relevance-ranked word search.
Everything marked strict was verified against the live sites (October 2026)
by opening result videos and reading **their own** tag / category lists (not
titles, and not the site-wide menus on video pages). URL schemes change, so
re-check before relying on an entry for automation.

**Strict** = the site intersects the terms (each result has all of them).
**Word search** = the terms are keywords; results are ranked by relevance and
are not guaranteed to match every term.

## Summary

| Site     | Tag + tag                   | Category + category   | Search + category                    | Exclude                |
| -------- | --------------------------- | --------------------- | ------------------------------------ | ---------------------- |
| XVIDEOS  | ❌ (keyword search)         | ❌                    | ❌                                   | ❌ (`-tag` is ignored) |
| PornHub  | ❌ (word search)            | ✅ strict, exactly 2  | ✅ strict, 1 category                | ✅ up to 10 categories |
| Eporner  | ❌ (word search)            | ✅ strict, any number | ✅ strict (search/tag + categories)  | ✅ categories          |
| xHamster | ❌ (word search)            | ❌                    | ❌                                   | ❌                     |
| XNXX     | ❌ (word search)            | — (no categories)     | ❌                                   | ❌                     |
| ThisVid  | ❌ (one tag only)           | ❌                    | ❌ (search + category is not strict) | ❌                     |
| YouPorn  | ❌ (word search)            | ❌                    | ❌                                   | ❌                     |
| Others   | not supported (word search) | not supported         | not supported                        | not supported          |

ThisVid's one strict combination — **one category + one tag** — does not fit
the columns above; see its section.

On every site, the in-app **Filter results** box (`word`, `-word`) can further
narrow the _loaded_ results on the device; that is a local text filter on
titles/tags, not a site query.

## XVIDEOS

- **No strict tag matching — not even for a single multi-word tag.** Tag pages
  are a keyword search over titles and tags:
  - `/tags/petite-brunette` lists **73,466** videos — more than `petite`
    (46,462) or `brunette` (50,095) alone, which a real tag could not. Of 8
    results opened, **none** carried a `petite-brunette` tag, only 2 had both
    `petite` and `brunette` tags, and some matched only through the title.
  - `/tags/milf+anal` and `/tags/redhead+big-tits+milf` behave the same: results
    were missing one or more of the tags (none of the 5 checked had all three).
  - `/tags/milf/anal` just redirects to the `anal` tag; `-tag` exclusion
    (`/tags/milf+anal+-lesbian`) is ignored.
  - The counts confirm it. Results-page counts: `milf` 48,587, `anal` 59,259,
    `milf+anal` **40,951** — that would make 84% of all MILF videos also anal;
    a real intersection would be a small fraction of either. `petite-brunette`
    (= `petite+brunette`) shows 26,078 against `petite` 32,679 (80%). The
    tag-index badges are worse: `milf-anal` 122,495 is larger than `milf`
    (73,089) and `anal` (95,337) put together. The search box is OR-like:
    `/?k=milf` 82,413 → `/?k=milf+anal` 135,138.
  - Single-word tags (`/tags/milf`) are the closest to a real tag listing.
- **Categories** (`/c/Femdom-235`, ~38 of them) cannot be combined with each
  other, with tags, or with a search: `?c=`, `?tag=` and `?k=` on a category
  page are ignored, `/c/X/tags/Y` is a 404. Category pages do support sort /
  length / quality as path segments: `/c/s:uploaddate/d:10-20min/q:hd/Femdom-235`.
- **Search + tag**: not possible (`/?k=…&tag=…` and `/tags/x?k=…` ignore the
  extra parameter).

**In the app:** Combine runs a word search (`/?k=milf+anal`); results are
relevance-ranked, not strict. Use the **Filter results** box to drop loaded
results by word.

## PornHub

- **Categories — strict, exactly 2 (straight, gay and trans).**
  `/video/incategories/milf/anal` (paging `?page=N`, sort `&o=mv` etc. work).
  Verified: 10 of 10 results (from the page's `#incategoryVideos` grid) list
  both MILF and Anal in their own categories.
  - Three categories (`/incategories/a/b/c`) → 404.
  - Gay: `/gay/video/incategories/amateur/bareback` (catalog slugs without
    their `-gay` suffix). Verified: 8 of 8 results list both categories.
  - Trans has no category index of its own (`/transgender/categories` is a
    404); a trans category is `/video/incategories/<category>/transgender`.
    Verified: 8 of 8 `milf/transgender` results list both MILF and
    Transgender. Most of the 111 straight categories have such a page; a few
    pairs (BBW, Casting, Bisexual Male, Scissoring, Described Video) are 404s,
    and the app falls back to a word search for those.
  - Lesbian "categories" are not categories: every entry on
    `/lesbian/categories` links to a search (`/lesbian/video/search?search=anal`),
    so lesbian combinations are word searches.
- **Search + 1 category, minus up to 10 categories — strict.** The search
  page's own filter (the category include / exclude lists) uses numeric
  category ids:
  `/video/search?search=redhead&filter_category=29&exclude_category=35-8`
  (29 MILF, 35 Anal, 8 Big Tits).
  - Verified: 8 of 8 results list MILF and neither Anal nor Big Tits in their
    own categories; the same holds on page 2 with a sort (`&o=mv&page=2`).
  - Several exclusions are **dash**-separated (`35-8`), at most 10. A comma
    (`35,8`) keeps only the first one.
  - Only **one** `filter_category`.
  - `?search=` on a category page is a 404.
- **Category page minus categories — strict.** `exclude_category` also works
  on category listings, `/video?c=29&exclude_category=8` (8 of 8 MILF results
  without Big Tits, where the unfiltered page is mostly Big Tits), and on
  `/video/incategories/milf/anal?exclude_category=8`. `filter_category` is
  ignored on category pages, and `incategories` only takes slugs (ids 404).
- **Tags**: no tag-intersection URL; multiple tags are a word search.

**In the app:**

- Two categories: PornHub category picker → **Combine** → pick 2 (Straight or
  Gay) → _Browse 2 together_. On Lesbian and Trans, Combine runs a word search.
- The picker loads PornHub's live catalog (111 straight, 53 gay, 73 lesbian
  entries). The **Trans** tab lists the straight categories, each opened as
  that category ∩ Transgender; _All trans_ is the whole trans section.
- Search + categories: run a search, open the **Categories** panel under the
  filters, tap a category once to require it (one at most), and tap others
  twice to exclude them (up to 10).
- On a category page the same panel only excludes.

## Eporner

- **Search or tag + categories — strict, include and exclude.**
  `/tag/redhead/?include=milf,anal&exclude=lesbian` (also on category pages,
  `/cat/japanese/?include=milf`).
  - Paging and sort: the page number goes **before** the sort segment —
    `/tag/redhead/2/longest/?include=milf`. (`/longest/2/` redirects to page 1.)
  - Verified: 8 of 8 `?include=milf` results and 5 of 5
    `?include=milf&exclude=anal` results had the right categories on their
    own pages.
  - Only **categories** (the ~86 on `/cats/`) work in `include`/`exclude`;
    tag slugs (`femdom`, `pantyhose`) are silently dropped.
- **Tag + tag**: not strict. Every multi-tag form (`/tag/redhead+milf/`,
  `/search/redhead milf/`, `/tag/redhead-milf/`) lands on a keyword-search page
  (`/tag/redhead-milf/`). The search narrows with each word (API totals:
  `redhead` 100k+, `redhead milf` 52,597, `redhead milf anal` 16,993), so it
  behaves AND-like, but it is keyword matching, not tag intersection.
- Tag slugs are case-sensitive: `/tag/czech-ouQ9EM84/` works, lowercase 404s.

**For a strict Eporner match:** search or open a tag, then use the
**Categories** panel on the results page — tap once to require a category,
twice to exclude it. Required categories are strict; the search term itself is
a keyword match.

## xHamster

- **No strict combination of any kind.**
  - `/categories/milf/anal` → 404; there is no multi-tag or multi-category URL.
  - Multiple terms only work as a word search: `/search/redhead+milf`
    (the form xHamster's own "related searches" use).
  - A search can only be narrowed by orientation (`?orientations=gay`), plus
    sort/length/quality.
- Single tags (`/tags/<slug>`) and categories (`/categories/<slug>`) are real
  listings with sort (`/newest`, `/best`), HD (`/hd`) and paging (`/2`).

**In the app:** Combine runs a word search; results are relevance-ranked.

## XNXX

- **No strict combination.** XNXX has no tag pages: `/tags/milf` redirects to
  the search `/search/milf`, and `/tags/milf/anal` redirects to `/search/anal`.
- `/search/big-tits` (hyphenated, as XNXX's own tag links use) ranks
  differently from `/search/big+tits` (separate words); neither is a strict
  match. The app uses the hyphenated form for a single tag.

**In the app:** Combine runs a word search.

## ThisVid

- **Category + one tag — strict.** `/categories/amateurs/?tag=anal` (or the
  other way round, `/tags/anal/?category=amateurs`) returns only videos in that
  category **and** with that tag. Checked by opening result pages: every one
  listed both (amateurs + anal, asian + massage). Paging works
  (`/categories/amateurs/2/?tag=anal`).
- **Only one tag.** `?tag=anal&tag=bdsm` keeps just the last tag; comma or `+`
  lists (`?tag=anal,bdsm`) are 404s; `?tag[]=…` is a server error.
- **Tag + tag**: no. `/tags/anal/?tag=bdsm` _replaces_ the tag (results are
  bdsm, not anal); `/tags/a,b/`, `/tags/a+b/`, `/tags/a/b/` are 404s.
- **Search + category**: not strict. `/search/?q=wife&category=amateurs`
  reorders the results but still returns other categories.

**In the app:** open a ThisVid category, type a tag in **Narrow by tag**
(autocomplete from ThisVid's A–Z tag index) and tap Apply — a strict
category + tag listing, with sort and paging. Combining several ThisVid tags
in the tag browser still runs a word search.

## YouPorn

- **No strict combination.** `/category/milf/anal/` is a 403; `category_id=` and
  `tag=` on search, tag or category pages are stripped by a redirect.
- Its "porn tags" are search aliases: `/porntags/redhead-milf/` returns exactly
  the same videos as `/search/?query=redhead+milf`, and `/porntags/redhead/`
  the same as `/search/?query=redhead`.

**In the app:** Combine runs a word search.

## Other sites

ThotHub, RedGifs, Reddit and the cam sites were **not tested** for combined
queries. In the app, combining tags on them runs a word search (the site's
normal search with all the words).

## How the app decides

Each site advertises `BrowseOptions.combine_tags` (how many tags one URL can
intersect exactly: PornHub 2, others 0) and `include_exclude` (Eporner,
PornHub). `max_include` / `max_exclude` cap the panel (PornHub 1 / 10), and
`exclude_only_kinds` lists listings that take only exclusions (PornHub
category pages). A combined tag slug is sent as `a,b,…`; when the count exceeds the
site's exact limit, `AppState::browse` turns it into a word search, and the
results page shows a note that the combination is not exact.
