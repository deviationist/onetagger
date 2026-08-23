# TODO

Wishlist for this fork. Items graduate to a `PLAN-*.md` when they are worth
designing properly.

## Make Manual Tag's cross-source merge discoverable

**The merging already works** — this is a UI problem, not a missing feature.

Ticking several matches in the Manual Tag dialog merges them:
`manual_tagger_apply` (`crates/onetagger-autotag/src/lib.rs:1325`) takes the
first selected match as the base and folds the rest in with `Track::merge`,
which is first-wins gap-filling (`self.field.or(other.field)`) for scalars —
including `art` — and a union for arrays like artists, genres and styles.

So the case that prompted this, *"Beatport has the metadata but no cover, another
source has the cover"*, is already handled: tick Beatport, then tick the source
with the art, and you get Beatport's fields plus the other's cover.

Two things make it invisible:

1. **Nothing says selecting several merges them.** It reads as "pick one", so
   nobody tries.
2. **Selection order decides precedence** and is not shown. The base is
   whichever match was ticked *first* (`selected.value.push(match)` in
   `ManualTag.vue:216`, then `matches.remove(0)`). Tick them in the other
   order and a different source wins every contested field — silently.

Worth doing:

- Say what will happen: a line in the dialog when 2+ are selected, e.g.
  *"Fields are taken from the first selected match; later ones fill the gaps."*
- Make the order visible and preferably changeable — number the selected
  matches, or let them be dragged.
- Consider showing which fields each match would actually contribute, so the
  cover-art-only case is obvious at a glance.

## Feature flags to hide unused views

Hide **Audio features** and **Auto rename** from the nav — never used here, and
they are two of six top-level tabs.

Notes:

- The tabs are `q-route-tab`s in `client/src/App.vue`; the routes themselves are
  in `client/src/scripts/router.ts`. Hiding the tab is not enough on its own —
  the route stays reachable by URL, which is fine for a tidy-up but not if the
  intent is ever "this build does not ship that feature".
- Decide the mechanism deliberately, because it sets a precedent:
  - **Settings toggle** — per-user, no rebuild, discoverable in the UI, and the
    view stays available for the one time someone wants it.
  - **Build-time / env flag** — genuinely removes it, but needs a rebuild to
    change and adds a build dimension to test.

  A settings toggle is probably right for "we never use these"; an env flag is
  for "this deployment must not expose them".
- Whichever is chosen, `Index.vue` and any deep links into the hidden views
  need a sane fallback rather than a blank page.
