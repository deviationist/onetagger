# Plan: pick metadata per field, across sources

**Status: explored, not started. Written 2026-08-24.**

## The problem, from a real track

`main/House/Savage Spirit - Afrodite (Gambaphro Mix).aiff`, 8:33. Beatport has
the correct release and **no cover art**. Other matches carry art but their
metadata is wrong. Today you choose a winner per *match*; what you want is a
winner per *field*.

## What already happens, which is half the answer

Ticking several matches already merges them. `Track::merge` is:

```rust
self.field = self.field.or(other.field);   // scalars: FIRST selection wins
merge_array(&mut self.field, other.field); // arrays:  UNION, in selection order
```

So for **scalars** the existing behaviour is nearly right for this case: select
Beatport first, then the match that has art, and because Beatport's `art` is
`None` the second one's art fills the gap. `album`, `label`, `bpm`, `key`,
`catalog_number`, dates and `thumbnail` all behave the same way.

**Arrays are where it breaks.** `artists`, `album_artists`, `genres`, `styles`,
`remixers` and `other` are *unioned*, not overridden. Taking art from a wrong
match therefore also inherits its artists and genres into the result. You cannot
borrow one field from a source without importing its lists.

That asymmetry is the actual defect, and it is invisible in the UI: nothing
tells you that ticking a second match will add its genres.

## The proposal

A second view on the same results — the list stays, this is an alternative
rendering, not a replacement.

- **Rows**: the fields worth choosing between (below).
- **Columns**: one per match, headed by platform and accuracy.
- **Cells**: the value that source offers, with a radio to select it.
- **Column header**: selects that source for every row at once, which makes the
  common case ("take Beatport, override the art") two clicks.
- **Empty cells** are not selectable and should read as absent rather than
  blank, since "this source has no art" is the fact that sends you looking at
  another column.

### Rows worth showing

Scalars, which the matrix can genuinely arbitrate:

    title · version · album · album artist · label · catalogue number
    key · bpm · release date · publish date · ISRC · mood · explicit
    art · thumbnail · URL

Arrays need a different control, because "pick one source" is the wrong verb:

    artists · album artists · genres · styles · remixers

For these, per-source **checkboxes** rather than radios, so the union is chosen
deliberately rather than inherited. Defaulting to the primary source's values
only would already be an improvement on today.

## The hard part is the backend, not the UI

`manual_tagger_apply(matches, path, config)` takes a *list of matches* and
merges them by precedence. There is no way to say "album from here, art from
there". Three ways out:

1. **Compose client-side, send one synthetic match.** The client already holds a
   full `Track` for every result, so it can build the composite and send a
   single-element list. Risk: the backend calls `extend_track` on it first,
   using `track.platform` — which for a composite is a lie, and may re-fetch and
   overwrite the very fields that were chosen. Needs either a flag to skip
   extension, or extension to run *before* composition.
2. **New action carrying a field→source map.** Honest and explicit; the backend
   composes. More surface, and duplicates knowledge of the field list on both
   sides.
3. **Extend precedence to per-field overrides.** Keep the current call, add an
   optional `overrides: HashMap<String, usize>` naming which match index wins
   for which field. Smallest change to the protocol; the merge loop consults it
   instead of blanket `.or()`.

(3) looks best: it leaves today's behaviour as the default, and the override map
is empty for every existing caller.

## Open questions

- ~~**Does `extend_track` need to run per source before composing?**~~
  **Answered 2026-08-24: yes, for two platforms.** `musicbrainz.extend_track`
  sets `track.art` from the Cover Art Archive when the release has a front or
  back cover, and `traxsource` takes an `album_art` parameter. Every other
  platform sets art during search or not at all.

  So a matrix drawn on search results **understates** what those two can offer:
  a MusicBrainz row would show an empty art cell for a release that would in
  fact gain a cover on apply. The grid must either extend each match before
  drawing -- one request per match, on a screen the operator is already waiting
  at -- or mark those cells "unknown until applied" rather than "none", because
  an empty cell is precisely the thing that sends you to another column.

  Measured on `Savage Spirit - Afrodite (Gambaphro Mix)`, working on a copy:
  three matches came back (musicbrainz 100% 6:03 no art, musicbrainz 75% 6:19
  no art, discogs 100% 2:47 with art) against a file of 8:33. Applying the
  art-less MusicBrainz match left the file with zero APIC frames, which looked
  like proof that extension never fetches art -- it was not. That release simply
  has no cover in the Archive. One negative sample cannot show that a
  conditional branch never runs.

  Note also what the run says about the premise: none of the three matches is
  anywhere near 8:33, and Beatport -- where the good metadata was found by hand
  -- returned nothing at all in this run, while beatsource errored on a network
  request. Whatever the matrix does, it can only arbitrate between matches that
  came back.
- **Is `other` (arbitrary frames) worth exposing?** It is a list of
  `(FrameName, Vec<String>)` and could be dozens of rows.
- **How wide is too wide?** Five matches × 16 fields is a big grid. Probably
  worth capping visible columns and letting the rest scroll.

## Why this is worth doing rather than working around

The workaround is to tick matches in the right order and accept the array
pollution. That is invisible, order-dependent, and wrong in a way nobody would
notice: genres quietly accumulate from sources the operator rejected.
