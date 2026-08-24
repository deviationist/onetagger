# TODO

Wishlist for this fork. Items graduate to a `PLAN-*.md` when they are worth
designing properly.

## Dynamic tab title

The document title is the constant "One Tagger", so every tab and window looks
identical in the tab strip, the taskbar and the window switcher. With Quick Tag
and Edit Tags open side by side -- the normal way this fork gets used -- there
is nothing to tell them apart.

Make it reflect what is on screen. Roughly, most specific part first, since tab
strips truncate from the right:

    ALT (Interlude) -- Edit Tags -- OneTagger      (a file is open)
    Needs-attention -- Edit Tags -- OneTagger      (a folder, no file)
    Quick Tag -- OneTagger                         (no folder context)

Worth folding in: unsaved changes could show as a leading marker, the way
editors do, which would make an unsaved window findable without clicking
through them.

Spelling: the project uses both. `OneTagger` wins 85 occurrences to 11, and
owns the repo, crates, binary and image tag; the spaced `One Tagger` survives
in the user-visible strings -- `<title>`, the native window title, the
window-size warning, two tooltips. Use `OneTagger` in the new title and leave
the existing strings alone: changing them is a cosmetic diff that would
conflict on every rebase against upstream for no functional gain.

Client-side only -- a watcher on the view, folder and open file writing
`document.title`. No backend involvement, and it is generic rather than
homelab-specific, so it belongs in the image.

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
