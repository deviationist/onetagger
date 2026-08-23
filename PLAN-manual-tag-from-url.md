# Plan: "Tag from URL" — human-guided matching when autotag fails

**Status: postponed (2026-08-23).** Testing showed the existing Manual Tag
feature already handles most failures well, so this is parked rather than
dropped. Written down so it can be picked up cheaply later.

## Problem

When a track fails to autotag — OneTagger searches Discogs/Beatport/etc. and
finds nothing — the operator often *can* find the correct release by hand.
Today there is no way to hand that answer back to OneTagger, so the fallback
is filling tags in manually, which is tedious and doesn't scale.

Goal: paste the URL of the release/track you found, and let OneTagger do the
enrichment from there.

## What already exists (checked in the tree, 2026-08-23)

The plumbing is largely built — only the "point me at this exact item" entry
point is missing.

1. **`matchById`** — `client/src/components/AutotaggerAdvanced.vue:69`,
   labelled *"Track or Release ID tag as input to get exact match"*.
   When on, Discogs reads `DISCOGS_RELEASE_ID` from the file's own tags and
   fetches that release directly (`crates/onetagger-platforms/src/discogs.rs:155`).
   Beatport reads `BEATPORT_TRACK_ID` (`beatport.rs:290` — actually
   unconditional, not gated on the flag). No other platform participates.
2. **Manual Tag dialog** — `client/src/components/ManualTag.vue`, opened from
   Quick Tag. Runs every enabled platform against one file in parallel, lists
   all candidates with accuracy + reason badges, multi-select across platforms,
   then `manual_tagger_apply` (`crates/onetagger-autotag/src/lib.rs:1293`) does
   extend → merge → write. **This works well** — it is the base to build on.
   Its only gap: it is search-driven off title/artist, so an unfindable track
   yields "No results!".
3. **`MatchReason::ID`** — already a first-class match reason
   (`crates/onetagger-tagger/src/lib.rs:366`), already rendered in both the
   status table and the Manual Tag list.

**Today's workaround, no code needed:** Tag Editor → set `DISCOGS_RELEASE_ID`
to the release number → enable *matchById* → re-run. Works, but requires typing
the ID per file, and inside a multi-track release still relies on track-number
or fuzzy-title matching to pick the right track.

**What does not exist anywhere:** URL parsing, or any way to name an exact item
without going through search.

## Proposal

Extend Manual Tag with one new way to populate the candidate list it already
renders. Paste `discogs.com/release/12345` → backend asks each enabled platform
"is this URL yours?" → the owning platform fetches by ID → the release's tracks
appear as candidates badged `ID` instead of a fuzzy percentage → tick one →
apply, via the existing unchanged path.

### Backend — three additions, all with default impls (no ABI break)

- `AutotaggerSourceBuilder::parse_url(&self, url) -> Option<ItemId>`,
  default `None`. Each platform recognises its own URL shapes.
- `AutotaggerSource::fetch_by_id(&mut self, id: &ItemId, config) -> Result<Vec<TrackMatch>, Error>`,
  default `Err(unsupported)`. Release URL → all tracks on the release as
  candidates; track URL → one. Reason `MatchReason::ID`.
- New socket action `ManualTagUrl { url, path, config }` alongside the existing
  `ManualTag` / `ManualTagApply` (`crates/onetagger-ui/src/socket.rs:97-98`).

Default impls matter for plugins: the FFI wrapper
(`crates/onetagger-tagger/src/custom.rs`) only exports `_1t_match_track` and
`_1t_extend_track`, so custom platforms silently report "URL not supported"
rather than failing to load.

### Frontend

A text input in `ManualTag.vue` (made prominent when the search returns
"No results!") plus one branch in `client/src/scripts/manualtag.ts`. The
results handler and the apply path are reused verbatim.

### Per-platform cost

| Platform | Effort | Why |
|---|---|---|
| Discogs | ~free | `full_release(ReleaseType, id)` + `get_track(i, cfg)` already exist. URLs: `/release/123`, `/master/456`, `/<lang>/release/123-Slug` |
| Beatport | ~free | `self.track(id)` + the only `get_album` impl in the tree (`beatport.rs:410`) |
| MusicBrainz, Spotify, Deezer, iTunes | small | all have ID-addressable APIs |
| Juno, Traxsource, Bandcamp, Beatsource, BPMSupreme | skip | scraper/search-only, no ID lookup |

Ship **Discogs + Beatport** first — that is the actual failure surface. The
rest return "not supported" via the default impl.

## Design decision worth keeping

**Force-enable the Track ID / Release ID tags on the URL path.**
`Track::write_to_file` already writes `{PLATFORM}_TRACK_ID` /
`{PLATFORM}_RELEASE_ID` when those tags are enabled
(`crates/onetagger-autotag/src/lib.rs:228-235`). If the URL path always writes
them, a manually-guided match becomes **self-documenting**: any later re-run
with `matchById` on re-finds the same release with no human in the loop. Costs
nothing, turns a one-shot fix into durable metadata — which matters for the
autotag-bridge pipeline.

## Open questions

1. **Entry point** — is Quick Tag → Manual Tag enough, or does the Autotagger
   status table also need a per-failed-row "tag from URL" action?
   Leaning: Quick Tag only. With `on-hold/Needs-attention/` as the staging
   bucket for failures and publish/recheck buttons already there, the natural
   loop is: track lands in Needs-attention → open in Quick Tag → paste URL →
   tag → publish. That makes the status-table entry point redundant.
2. **Discogs master URLs** — auto-resolve to `main_release`, or show the
   release list and let the operator pick? Leaning: auto-resolve, with the
   candidate list as fallback.
3. **Track selection inside a multi-track release** — run the URL-fetched
   tracks through the existing `MatchingUtils::match_track` purely to
   pre-select and sort the likely one, while still showing all? Leaning: yes,
   keeps the interaction to paste-and-confirm.
4. **CLI / headless entry point** — is a `onetagger-cli tag-url --path … --url …`
   subcommand wanted, so overrides can be scripted from autotag-bridge rather
   than only clicked? Separate, small, additive.

## Implementation note

Do this in a git worktree — the main checkout has had concurrent agent work
(`beatport.rs` was dirty when this plan was written).
