# Plan: auto-refresh the file browser when a folder changes on disk

**Status: proposed (2026-08-23).** Not started.

## Problem

The Tag Editor and Quick Tag file browsers load a folder once and never look
again. When something outside OneTagger adds or removes a file in the folder
you are looking at, the listing silently goes stale — you keep seeing a track
that is no longer there, clicking it fails, and the only way back is a manual
reload of the whole page.

This matters when an external pipeline is moving files in and out of the same
tree OneTagger is pointed at, which is precisely the "fix the metadata by hand,
then let the pipeline take it" workflow.

Two things are wanted:

1. The listing keeps up with the folder on its own.
2. If the file currently **open in the editor** disappears, say so, rather than
   leaving a form bound to a path that no longer exists.

## What already exists (checked in the tree, 2026-08-23)

Most of part 2 is built, for the delete feature.

- **`onDeleted(paths)`** — `client/src/views/TagEditor.vue:531`. Given paths
  that have gone, it clears `file.value` and `changes` when the open file is
  among them (the comment notes exactly the hazard: autosave in `loadFile()`
  would otherwise write to a moved file), prunes `customList`, refreshes
  whichever listing is on screen — `runLibrarySearch()` when a search result
  set is displayed, `loadFiles()` otherwise — and notifies.
  **This is the handler auto-refresh should feed, not a new one.**
- **`tagEditorFolder`** — `crates/onetagger-ui/src/socket.rs:629`. Stateless
  request/response: send a path, get `{files, path, recursive}` back. The
  client handler at `TagEditor.vue:922` replaces `originalFiles` wholesale.
- **`quickTagLoad`** — the Quick Tag equivalent, same shape.
- **No `notify` crate** anywhere in the workspace, so a server-side filesystem
  watcher would be a new dependency.

## Options

### A. Server-side filesystem watcher, pushed over the existing socket

Add `notify`, watch whatever folder the connection last asked for, push an
update when it fires.

**Rejected.** Inotify does not see changes a *different* machine makes to a
network filesystem — it only reports what passes through the local kernel's
VFS. A tree shared over NFS/SMB therefore reports some changes and silently
misses others depending on which host made them, which is a worse failure than
not refreshing at all: it looks like it works. (The same reasoning is why the
external watcher feeding this pipeline polls instead of using inotify.)

It is also the largest change: a new dependency, plus per-connection state
tracking which folder to watch and tearing it down on navigate/disconnect.

### B. Client polls `tagEditorFolder` and re-applies unconditionally

Simplest to write, and wrong. The existing handler replaces the list wholesale,
so every tick would reset scroll position, re-apply sort and filter, and fight
the user. Also ships a full listing over the wire on every tick whether or not
anything changed.

### C. Client polls a cheap signature, reloads only on change ← **recommended**

Poll the **folder's own mtime**, which is exactly the right signal: adding,
removing or renaming an entry updates the parent directory's mtime, while
merely editing a file's contents does not — and "added or removed" is the whole
requirement. One `stat()` per tick instead of a directory walk.

New action, mirroring the shape of the existing ones:

```
-> { action: "folderSignature", path }
<- { action: "folderSignature", path, mtime, count }
```

`count` guards the one case mtime misses: two changes inside the same
filesystem timestamp granularity. Cheap, since it comes from the same call.

On a change, call the *existing* `loadFiles()` / `runLibrarySearch()` path so
there is one code path for "re-read this folder", already search-aware.

## Design details

### Polling

- **Interval: 10 s**, and only while the view is mounted **and**
  `document.visibilityState === 'visible'`. A background tab polling a network
  filesystem forever is pure waste.
- Stop polling while a modal/dialog is open, to avoid the list moving under a
  dialog that refers to it.

### Latency is bounded by the filesystem, not by the interval

On an NFS mount with default attribute caching (`acdirmin=30`, `acdirmax=60`),
the client may serve a cached directory mtime for up to 60 s, so a change made
on **another** host can take that long to become visible no matter how fast we
poll. Changes made by a process on the *same* host — which includes the
pipeline that prompted this — invalidate the local cache immediately and show
up on the next tick.

Worth stating plainly in any UI copy: this is "keeps up", not "instant". Do not
chase the gap by lowering the interval; the fix would be `actimeo=0` on the
mount, which is a much worse trade.

### Preserving what the user is looking at

Re-applying a listing must not feel like a page reload. On refresh, preserve:

- scroll position of the browser pane,
- the current selection / open file,
- sort mode and direction, filter text, and search state.

Sort and filter already live in refs the load path re-applies. Scroll is the
one that needs explicit save/restore around the list swap.

### When the open file disappears

Feed the vanished paths into the existing `onDeleted()` — with one deliberate
divergence.

`onDeleted()` currently clears `file.value` and `changes` unconditionally. That
is right for a **user-initiated** delete: they asked for it, and pending edits
are moot. It is wrong for a file that vanished **externally**, because the user
did not ask, and silently discarding their unsaved edits is the one outcome
they cannot recover from.

So:

| open file gone, and… | behaviour |
|---|---|
| no pending changes | clear the form, notify — today's `onDeleted()` behaviour |
| pending changes | **keep the form populated**, disable Save, show a persistent warning |

The warning must not claim the file was deleted. A directory listing cannot
distinguish "moved away" from "removed" — both are simply absence. Wording
should be along the lines of *"This file is no longer in this folder. Your
unsaved changes are still here but cannot be saved to it."*

That gives the operator a chance to copy values out, which clearing does not.

### Quick Tag

Same mechanism, but Quick Tag holds unsaved state per track
(`track.isChanged()`) and can have several tracks selected, so the "pending
changes" branch above matters more there. Worth doing second, once the Tag
Editor version has settled.

## Cost

One `stat()` per client per 10 s, plus a full folder load only when something
actually changed. Negligible next to the existing per-entry `stat()` a
directory listing already performs (~2.9 ms for 751 entries).

Option B, by contrast, would ship roughly 150 KB of JSON every tick per client
regardless of whether anything happened.

## Open questions

- Should the refresh be **silently** applied, or should an unobtrusive
  indicator show that the listing moved? Silent is less noisy but can be
  disorienting if a row shifts under the cursor mid-click.
- Should polling extend to the **recursive** / custom-list modes, where "the
  folder" is several folders? The signature would need to cover all of them,
  and the cheap single-`stat()` property is lost.

## Out of scope

- Watching folders the user is not currently looking at.
- Detecting *content* changes to files (tags rewritten underneath us). Folder
  mtime deliberately does not signal that, and the current listing does not
  display anything that would go stale as a result.
