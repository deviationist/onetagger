# Plan: getting this fork's work upstream

## Websocket + player fixes, ready to become PRs (2026-08-24)

Four defects found while debugging a frozen tab, all verified present on
`upstream/master`, all fixed on `main` in commit `e8f9685` (plus `6192119`
for the deep-link half, which is ours and not upstream's concern).

| Candidate branch | What it fixes | Adaptation needed |
|---|---|---|
| `pr/fix-empty-websocket-frame` | Close frame parsed as JSON -> spurious EOF error + send-after-close | **cut, done** |
| `pr/fix-websocket-reconnect` | Client never reconnects, so any drop bricks the page until reload | applies to upstream's `onetagger.ts` roughly unchanged |
| `pr/fix-player-without-audio-device` | `AudioPlayer::new` panics with no sound card; `seek()` unwraps the resulting closed channel | must be rewritten for upstream's older rodio -- `OutputStream::try_default()` + `Sink::try_new()`, two unwraps rather than our one |

The reconnect one is the headline: it turns every transient drop into a
permanent freeze that a reload silently fixes, which is exactly the shape of
bug that generates unreproducible reports for years. Worth splitting three
ways rather than bundling -- small independent PRs merge, grab-bags stall.

**Status: deferred (2026-08-23).** Deliberately last. The fork exists to fit
this library's workflow, and that comes first; contributing back is what
happens once it does, not something to interleave with it.

Nothing here is urgent enough to change that -- including the path-confinement
item, which is a real vulnerability in *upstream's* shipped code but not in
ours, since this fork already closes it. Offering the fix is a courtesy to
other people running an exposed instance, not maintenance of our own.

Do not re-propose this while feature work is outstanding.

## Goal

Upstream has not published since February 2026, but if it revives, most of what
this fork adds is worth offering back — and some of it upstream arguably needs.
This is the order to do it in and the method, so contributing is a series of
small self-contained PRs rather than one unmergeable dump.

## The test

A change belongs on a `pr/` branch only if it reads **nothing that exists only
in this fork**. Everything else is `feature/`, however upstreamable the idea is.

**File provenance is a first filter, not the answer.** Checking whether a commit
touches files upstream does not have will catch a new module, but it misses the
common case: a change confined to upstream's own files that *calls* something of
ours. `feature/delete-for-real` touches six upstream files and nothing of ours —
yet its refresh path calls `searchQuery` and `runLibrarySearch()`, both from our
library-search work, so it cannot sit on `upstream/master` unchanged.

So the check is two-pass: file provenance first, then read the diff for symbols
that upstream does not have.

## The method: reduced variants, not cherry-picks

Nearly every addition here was built on top of an earlier one, so
`git cherry-pick` onto `upstream/master` mostly does not apply — and where it
applies it can quietly drag in a hunk that references our code.

For each candidate: branch from `upstream/master`, write the change again in
that context, drop the parts that only make sense here, and verify it compiles
against upstream before pushing:

```sh
git checkout -b pr/<name> upstream/master
# write the reduced variant
docker run --rm -v "$PWD":/src:ro -v cargo-cache:/usr/local/cargo/registry \
  -v target-upstream:/target -w /src -e CARGO_TARGET_DIR=/target \
  rust:1-bookworm bash -c \
  'apt-get update -qq && apt-get install -y -qq libasound2-dev lld >/dev/null; cargo check'
```

Compiling on the upstream base is the step that actually proves contributability;
a clean-looking diff does not.

## Candidates

Confidence is stated because only the first group has been verified by building
it; the rest is read from the diffs.

### Correctly based on `upstream/master`

Being on the right base is necessary, not sufficient: a branch can sit on
upstream and still have drifted behind what `main` actually ships.

| Branch | Notes |
|---|---|
| `pr/aiff-name-chunk-sync` | 1 commit over upstream |
| `pr/wss-https-scheme` | 1 commit over upstream |
| `pr/docker-support` | **Stale.** 3 commits over upstream, but 29 lines of `Dockerfile` and 46 of `docker-compose.yml` behind what `main` runs — it would contribute a setup we have outgrown. Refresh against `main` before offering it. |
| `pr/beatport-placeholder-ids` | 1 commit; verified compiling on the upstream base |

### Free wins — small, self-contained, pure bug fixes

Upstream has little reason to refuse these, and each should be its own PR.

- Beatport search after the site dropped `__NEXT_DATA__`
- Beatsource token handling and query encoding
- A malformed tag no longer making a track unplayable
- Duplicate custom tags prevented; ID3 date formats standardised
- Album art persisting correctly
- Text in the UI being selectable
- Quick Tag opening at the configured folder rather than its parent
- `stat()` once per browser entry instead of twice

Each needs finding in history and rewriting against upstream — they were not
developed as isolated commits.

### Highest value — do this one properly

**Path confinement (`feature/confine-file-access`).** Upstream takes the client
at its word about paths, so an exposed server will serve `/audio?path=/etc/passwd`
to anyone with a browser, and the websocket actions read, write, retag and delete
equally freely. This is a real vulnerability in shipped code, not a nicety.

Verified: the change touches five upstream files plus its own new `paths.rs`, and
one file of ours — `browsersort.ts` — which is incidental and can be dropped. So
a reduced variant is achievable, and it is the single contribution most worth the
effort.

Consider reporting it privately first rather than opening a public PR that
describes the hole and how to reach it.

### Needs a reduced variant — depends on our work

| Feature | Depends on |
|---|---|
| Tag Editor delete | library search (`searchQuery`, `runLibrarySearch`) |
| Delete-for-real (no OS trash) | shares the delete path above |
| Autotagger status table | `autotaggerStatus.ts`, `urlstate.ts` |
| Browser sort | `browsersort.ts`, and birth time plumbed through `browser.rs` |

The delete work is worth offering anyway — upstream can delete from Quick Tag
but not the Tag Editor, and the OS-trash fallback genuinely misbehaves on a
server. It just has to be rewritten without the search-aware refresh.

### Fork-only — do not offer

- Linkable views (`urlstate.ts`) is arguably upstreamable, but everything else
  here now reads it; extracting it is a bigger job than its value upstream.
- Anything in this repo's `PLAN-*.md` / `TODO.md` / ops notes.

## Suggested order

1. **Path confinement** — the only item with a security argument.
2. **The free wins**, cheapest first. They build goodwill and are individually
   trivial to review.
3. **Library search**, if it turns out to be self-contained — it unblocks the
   delete work.
4. **Tag Editor delete**, reduced.
5. Everything else, only if upstream shows signs of life.

## Mislabelled branches to settle first

- `pr/beatport-placeholder-art` — based on `main` (71 commits over upstream);
  superseded by `pr/beatport-placeholder-ids`.
- `pr/tageditor-browser-sort` — based on `main` (35 commits over upstream), so
  not contributable as-is despite the name.

Neither can be sent upstream. Renaming them to `feature/` would say what they
actually are, but rule 4 says delete nothing, and a rename drops the old ref —
so that is a decision to take deliberately rather than in passing.
