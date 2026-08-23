# TODO

Wishlist for this fork. Items graduate to a `PLAN-*.md` when they are worth
designing properly.

## Manual Tag merge discoverability -- done

Ticked matches now carry a numbered badge (#1 in primary, rest grey) with
tooltips saying which is the base, plus a line above Apply stating the rule.
No behaviour change -- the merge always worked; selection order deciding
precedence was the invisible part.

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

## Copy button on the filename -- done

Left of delete, copies the filename, disabled outside a secure context with a
tooltip saying why.

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
