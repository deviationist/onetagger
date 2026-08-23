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

## Guided metadata entry for the common fields

A small form -- Title, Artist, Album Artist, Album -- that writes the right
underlying tags in one save, instead of adding each frame and typing into it.

**This is a fast path, not new capability.** Worth being clear about, because
the parts already exist and the value is only in collapsing them:

- The Tag Editor already labels known tags in plain language: `TagEditor.vue`
  renders `ABSTRACTIONS[tag]` ("Title") with the raw frame beside it, so nobody
  is deciphering `TIT2` today.
- Adding a missing tag already works through the `TagField` autocomplete, which
  takes `:format='tagFormat'` and so offers the right names per format.

What is missing is doing four fields at once. Today each is: pick the tag, then
type the value, repeat.

**Match the completeness gate.** The pipeline holds a track back unless it has
title, artist, album, album_artist and artwork, so those exact fields are the
ones worth putting on the form -- it becomes the fastest way to clear a track
out of `Needs-attention`, which is where the friction actually is. Artwork is
the fifth and is not a text input; `AddAlbumArt` already covers it and should
sit alongside rather than be reimplemented.

Two things it must not shortcut:

- **Do not hardcode `TIT2` and friends.** `tagFormat` resolves to id3, vorbis
  or mp4, and the library holds AIFF and MP3 now but the editor is used on FLAC
  too. Write through the same per-format mapping `TagField` uses.
- **Go through the normal save path.** Writing a title is not just `TIT2` for
  AIFF: this fork keeps the IFF `NAME` chunk in step, and Plex reads `NAME` in
  preference. A form that wrote tags by a side route would produce files that
  look right in OneTagger and wrong in Plex -- the exact bug that sync exists
  to prevent.

Prefill from what the file already has, so it doubles as "show me the four
fields that matter" rather than only being an entry form.

## Copy button on the open file's name (Tag Editor)

Requested 2026-08-23. A copy-to-clipboard button beside the filename shown
above the tag list, to the **left** of the delete button.

The markup is `client/src/views/TagEditor.vue` around line 178:

```
<div class='text-subtitle2 ... selectable' title='Select to copy'>{{file.filename}}</div>
<q-btn round dense flat class='q-ml-sm' @click='confirmDelete(file.path)'>
```

so it is a `q-btn` inserted between those two, matching the delete button's
`round dense flat` styling with an `mdi-content-copy` icon and a tooltip.

It copies the **filename** -- the text shown -- not the full path. Decided
2026-08-23; `file.path` is available right there, so the temptation is to
"helpfully" copy that instead, and it is the wrong thing.

Two things to handle:

- **`navigator.clipboard` needs a secure context.** Over the HTTPS vhost that
  is fine, but a direct LAN visit to the server's own port is plain HTTP, where
  the API is simply absent and an unguarded call throws. Feature-detect and
  either fall back to the old select-and-copy or disable the button with a
  tooltip explaining why -- silently doing nothing is the bad outcome.
- **Confirm it happened.** The app already uses `$q.notify`; a short "Copied"
  matches how saving reports itself.

Also drop the now-misleading `title='Select to copy'` hint from the filename
once a real button exists.

## Feature flags to hide unused views -- done

Shipped as a `hiddenViews` list in settings, with toggles under
**Settings -> Views** for Audio Features and Auto Rename.

Hiding is real rather than cosmetic: the tab goes, the landing page stops
advertising the feature, and a router guard redirects a bookmarked or typed URL
home. Settings arrive over the socket after the first route resolves, so a
watcher leaves the view once they land -- without it, a cold load straight into
a hidden view would sit there.

Settings rather than an env var, deliberately: the guard reads it client-side
and works immediately, where an env var would have to reach the browser first
and reintroduce the same race the watcher exists to close. Settings also
persist into the config bind-mount, which is the durability an env var would
otherwise have bought.
