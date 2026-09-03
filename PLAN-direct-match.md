# Plan: Direct match — name the release yourself

**Status: built 2026-09-03**, on `feature/direct-match`. Supersedes
`PLAN-manual-tag-from-url.md`, which proposed the URL half alone and was parked
on 2026-08-23; both halves are here, behind one input.

## Problem

Autotag matches a file against platform search results. When the tags are wrong
enough that search returns nothing — or returns the wrong record confidently —
there is no way to hand OneTagger the answer. The operator can nearly always
find the right release on discogs.com in seconds; the fallback until now was
typing the fields in by hand, or writing `DISCOGS_RELEASE_ID` into the file and
re-running with `matchById`.

## What this adds

A third view in the Manual Tag dialog, beside List and Matrix, with one text
field that takes **either** a release URL **or** free-text search terms:

- paste `discogs.com/release/12345` → straight to that release's tracklist
- type `artist title` → a release picker, then the tracklist

Tick the track, and it joins the result set as `MatchReason::ID` with accuracy
1.0. From there nothing is new: the List merges it by precedence, the Matrix
arbitrates it field by field against whatever the search found.

### Why one field rather than two

The string says which it is. Asking the operator to first declare "URL" or
"search" is a question the input already answers, and getting it wrong costs one
cheap round trip: URL detection is deliberately loose on the client, and the
*backend* decides whether any platform claims it. Unclaimed falls back to the
search path with a message saying so.

### Why both halves, when the URL alone was the original proposal

They fail in opposite places. Discogs' own search has facets — format, year,
country, label — that a modal will never match, so when the operator has
*already* found the record, pasting the URL is both less work here and less code
to write. But when they have not, sending them out to a browser and back is
worse than a search box that is already in front of them. The shared spine is
`fetch_release`; the search path is one extra trait method on top.

## Shape

The load-bearing fact is that **the matrix is provenance-agnostic**:
`ManualTagMatrix.vue` takes `matches: TrackMatch[]`, `path`, `config` and
nothing else, `ManualTagExtend` resolves a builder from `m.track.platform`, and
`ManualTagApplyComposed` writes a composed `Track`. None of it cares how a match
was obtained. So the entire feature is: *produce `TrackMatch` values*. Everything
downstream — extend, matrix, compose, write — was reused unchanged.

### Backend

Two methods on the **builder**, two on the **source**, all with default impls:

| Where | Method | Default |
|---|---|---|
| `AutotaggerSourceBuilder` | `supports_direct_match()` | `false` |
| `AutotaggerSourceBuilder` | `parse_url(url) -> Option<String>` | `None` |
| `AutotaggerSource` | `search_releases(query, config)` | warn + `vec![]` |
| `AutotaggerSource` | `fetch_release(id, config)` | warn + `vec![]` |

The two URL-facing ones sit on the *builder* on purpose: the UI asks "whose URL
is this" while listing platforms, and building a source is not free — Discogs
validates its token over the network in `get_source`.

Defaults are mandatory, not politeness: the plugin FFI
(`crates/onetagger-tagger/src/custom.rs`) exports only `_1t_match_track` and
`_1t_extend_track`, so a custom platform has no way to answer any of these.

New platform-neutral `ReleaseCandidate` for picker rows. Socket actions
`DirectMatchSearch`, `DirectMatchUrl`, `DirectMatchFetch`; the two that touch a
platform run under `spawn_blocking`, because Discogs `sleep`s inside `get()` to
hold its own rate limit and the socket task must not be what sleeps.

### The capability flag does not go in `PlatformInfo`

`PlatformInfo` is `repr(C)` and is returned across the plugin FFI, so adding a
field changes the layout every already-compiled custom platform returns. The
flag went on `AutotaggerPlatformInfo` instead — the fork's own serde-only
wrapper, which is what the platform list is serialised from anyway.

### Discogs

Nearly free, as the earlier plan predicted: `search()`, `full_release()` and
`get_track()` all already existed. Three things did have to change.

**`ReleaseMasterSearchResult` was gutted.** Every field but `id`, `type` and
`resource_url` was commented out upstream — including `title`. Fine while the
only consumer is `match_track`, which fetches the full release anyway; useless
for a picker, where the operator is choosing *between* rows and two pressings of
one record are told apart by year, country, format and catalogue number. All
restored as `Option`, because Discogs omits them per result type and `year`
arrives as a string.

**Search is releases-only.** A master is a grouping: no catalogue number, no
country, no release date, and a tracklist borrowed from its main release. As a
tagging target it is strictly worse than the pressing underneath it, and listing
both would double every row. Master *URLs* are still accepted and resolved
through `main_release`, because people paste them constantly.

**Tracklists interleave headings with tracks.** A heading ("Side A") has no
`position`, so those are filtered — after `enumerate()`, since `get_track`
indexes into the untouched list.

### Client

`ManualTagDirect.vue`, a sibling of `ManualTagMatrix.vue`. `buildConfig()` moved
out of `ManualTag.vue` into `manualtag.ts` and is shared: the direct panel needs
the same credentials the search does, and two copies would drift.

`reset(keepDirect)` is the one piece of real state design. Hand-picked matches
are kept in `directMatches` and `matches` is re-seeded from it, so pressing Start
clears the previous *search* without throwing away a release the operator named.
That is what lets the two compose in either order — which is the entire argument
for putting direct match in this dialog rather than in one of its own.

## Decisions worth keeping

- **Reuse `MatchReason::ID`, don't add `Manual`.** A hand-picked release *is* an
  ID match — accuracy 1.0, no fuzzy score to report — and `ID` already renders as
  a badge in both the status table and the Manual Tag list.
- **Direct is always in the view toggle**, where List and Matrix appear only once
  there are matches. It is the answer to "No results!", so hiding it until there
  are results would hide it exactly when it is wanted.
- **Write the release ID back.** `Track::write_to_file` already writes
  `{PLATFORM}_RELEASE_ID` when that tag is enabled, so a hand-guided match is
  self-documenting: a later re-run with `matchById` re-finds the same release
  with no human in the loop. Nothing to build; just do not turn the tag off.

## Not done

- **Beatport.** `self.track(id)` and the tree's only `get_album` impl are both
  there, so it is a small addition — but Discogs is the actual failure surface
  and shipping one platform proves the seam.
- **Rate-limit feedback.** Discogs is 25 req/min unauthenticated, 60 with a
  token, enforced by sleeping. Search is on Enter, never per keystroke, which is
  enough at 60; without a token every action is a ~2.4 s stall and the UI says
  nothing about why.
- **A CLI entry point.** `onetagger-cli tag-url --path … --url …` would let
  media-bridge script an override rather than only clicking one. Additive.
