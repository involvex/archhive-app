# Combining tags, categories and searches

What each site can **strictly** combine in one listing, i.e. results that must
match _every_ term, versus what only works as a relevance-ranked word search.
Everything below was verified against the live sites (October 2026) by
comparing result sets; URL schemes change, so re-check before relying on an
entry for automation.

**Strict** = the site intersects the terms (each result has all of them).
**Word search** = the terms are keywords; results are ranked by relevance and
are not guaranteed to match every term.

## Summary

| Site     | Tag + tag                   | Category + category   | Search + category                    | Exclude                |
| -------- | --------------------------- | --------------------- | ------------------------------------ | ---------------------- |
| XVIDEOS  | ✅ strict, up to 5          | ❌                    | ❌                                   | ❌ (`-tag` is ignored) |
| PornHub  | ❌ (word search)            | ✅ strict, exactly 2  | ❌                                   | ❌                     |
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

- **Tags — strict, up to 5.** `/tags/milf+anal+big-tits` returns videos tagged
  with all of them; paging is `/tags/milf+anal/2`.
  - Tag pages ignore sort, date, length and quality filters. When any of those
    is set, the app runs the tags as a word search (`/?k=milf+anal&sort=…`)
    instead, which is **not** strict.
  - `/tags/milf/anal` is not a combination — it redirects to the `anal` tag.
  - Excluding with `-tag` (`/tags/milf+anal+-lesbian`) is ignored by the site.
- **Categories** (`/c/Femdom-235`, ~38 of them) cannot be combined with each
  other, with tags, or with a search: `?c=`, `?tag=` and `?k=` on a category
  page are ignored, `/c/X/tags/Y` is a 404. Category pages do support sort /
  length / quality as path segments: `/c/s:uploaddate/d:10-20min/q:hd/Femdom-235`.
- **Search + tag**: not possible (`/?k=…&tag=…` and `/tags/x?k=…` ignore the
  extra parameter).

**In the app:** tag browser → **Combine** → pick up to 5 tags → _Browse N
together_. Leave Sort/Length/Quality on the site default to keep it strict.
Combine is hidden on the Categories tab.

## PornHub

- **Categories — strict, exactly 2, straight only.**
  `/video/incategories/milf/anal` (paging `?page=N`, sort `&o=mv` etc. work).
  - Three categories (`/incategories/a/b/c`) → 404.
  - The gay / lesbian / trans sections have no such page (404).
- **Search + category**: not possible (`filter_category` on a search is
  ignored; `?search=` on a category page is a 404).
- **Tags**: no tag-intersection URL; multiple tags are a word search.

**In the app:** PornHub category picker → **Combine** → pick 2 (Straight) →
_Browse 2 together_. On other orientations Combine runs a word search.

## Eporner

- **Search or tag + categories — strict, include and exclude.**
  `/tag/redhead/?include=milf,anal&exclude=lesbian` (also on category pages,
  `/cat/japanese/?include=milf`).
  - Paging and sort: the page number goes **before** the sort segment —
    `/tag/redhead/2/longest/?include=milf`. (`/longest/2/` redirects to page 1.)
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

**In the app:** not wired up yet — combining ThisVid tags runs a word search.

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
intersect exactly: XVIDEOS 5, PornHub 2, others 0) and `include_exclude`
(Eporner). A combined tag slug is sent as `a,b,…`; when the count exceeds the
site's exact limit, `AppState::browse` turns it into a word search, and the
results page shows a note that the combination is not exact.
