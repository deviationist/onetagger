# TODO

Wishlist for this fork. Items graduate to a `PLAN-*.md` when they are worth
designing properly.

## Dynamic tab title -- done 2026-08-24

The tab is named after the open view: `Edit Tags - OneTagger`, `Quick Tag -
OneTagger`, and so on, falling back to plain `OneTagger` on the landing page.
Two windows side by side can now be told apart in the tab strip and the window
switcher, which is the normal way this fork gets used.

Route `meta.title` plus a `router.afterEach` hook, so a new view declares its
name where the route is declared and nothing else has to know. The view comes
first because tab strips truncate from the right.

Not done, and worth having if it ever irritates: the open **file** or folder in
the title, and a leading marker for unsaved changes the way editors do -- which
would make an unsaved window findable without clicking through them.

## Manual Tag merge discoverability -- done

Ticked matches now carry a numbered badge (#1 in primary, rest grey) with
tooltips saying which is the base, plus a line above Apply stating the rule.
No behaviour change -- the merge always worked; selection order deciding
precedence was the invisible part.

## Guided metadata entry -- done 2026-08-23

A "Common tags" form above the raw tag list: Title, Artist, Album Artist,
Album, prefilled from the file, applied and saved in one click. Artwork stays
where it was -- `AddAlbumArt` already covers the fifth field.

Tag names resolve through `tagFormat`, so the same form writes TIT2 on an AIFF
and TITLE on a FLAC, and the resolved names are shown beside the heading so it
is a shortcut past the frame IDs rather than a hiding of them.

Writes go through `onChange` and then `save()` -- the same path a hand edit
takes. That keeps the separator rule in one place, and keeps the AIFF NAME
chunk in step, which a side route would not have.

An empty field leaves the tag alone rather than deleting it; deletion stays on
the row below, where it needs a deliberate click.

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
