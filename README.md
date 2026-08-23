<p align='center'>
    <img alt='Logo' src='https://raw.githubusercontent.com/Marekkon5/onetagger/master/assets/onetagger-logo-github.png'>
</p>
<h1 align='center'>The ultimate cross-platform tagger for DJs</h1>

<h3 align='center'><b>
A fork of <a href='https://github.com/Marekkon5/onetagger'>Marekkon5/onetagger</a> &mdash; see <a href='#what-this-fork-adds'>what this fork adds</a>
</b></h3>

<h3 align='center'>
<a href='https://onetagger.github.io/'>Upstream website</a> | <a href='https://github.com/Marekkon5/onetagger/releases/'>Upstream releases</a>
</h3>
<br>

<p align='center'>
    <img alt='Supported OS' src='https://img.shields.io/badge/OS-Windows%2C%20Mac%20OS%2C%20Linux-orange'>
</p>

> This fork publishes no binaries of its own &mdash; build it yourself, or run the
> [Docker image](#docker). The release and build-status badges were removed rather than
> left pointing at upstream, where they described a different repository.

<h3 align='center'><b></b></h3>
<hr>

Cross-platform music tagger.
It can fetch metadata from Beatport, Traxsource, Juno Download, Discogs, Musicbrainz and Spotify.
It is also able to fetch Spotify's Audio Features based on ISRC & exact match. 
There is a manual tag editor and quick tag editor which lets you use keyboard shortcuts. Written in Rust, Vue.js and Quasar.

MP3, AIFF, FLAC, M4A (AAC, ALAC) supported.

*For more info and tutorials check out our [website](https://onetagger.github.io/).*

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4


## What this fork adds

This is a fork of [Marekkon5/onetagger](https://github.com/Marekkon5/onetagger). It tracks
upstream and keeps every change on top; nothing upstream is removed.

Upstream has not published a commit since February 2026, so this fork is where these
changes live. The `upstream` remote is still tracked, and if the project revives its work
merges straight in.

If you run OneTagger **on a server** rather than on your desktop, or you keep a large
library in **AIFF**, this fork is likely worth using.

**Runs headless, in Docker**

- A `Dockerfile` and `docker-compose.yml` that build `onetagger-cli` only, so there is no
  GUI/webkit/gtk dependency chain. The web UI is embedded and served on port `36913`.
- Built on the [LinuxServer.io](https://www.linuxserver.io/) s6-overlay base, so it takes
  the usual `PUID` / `PGID` / `UMASK` environment variables instead of needing `--user`
  juggling or a umask wrapper. See [Docker](#docker) below.
- The client speaks `wss://` when it is served over HTTPS, so it works behind a TLS
  reverse proxy instead of failing to open its websocket.
- The audio endpoint honours HTTP range requests, so seeking in the player works when the
  UI is reached over a network rather than from localhost.

**Fixes AIFF titles that never reach Plex (and other `NAME`-preferring players)**

An AIFF can carry a title in two independent places: the IFF `NAME` chunk near the start
of the file, and an ID3 `TIT2` frame in the `ID3 ` chunk at the end. Nothing in the spec
says which wins, and consumers disagree. Upstream writes only ID3, so editing a title
leaves the file self-contradictory — and any player that prefers `NAME` keeps showing the
old title through any number of rescans.

Measured over a 2740-track AIFF library, Plex used `NAME` in 1718/1718 files that had one,
and `TIT2` in 1022/1022 that did not, with no counterexamples. This fork keeps the two in
step on write. Only an existing `NAME` is updated — a file without one is unambiguous
precisely because every consumer falls back to ID3, so creating one would manufacture the
problem rather than prevent it. The file keeps its inode, so a filesystem watcher sees an
edit rather than a delete followed by a create.

**Search the whole library, not just the current folder**

Quick Tag and the Tag Editor can both search every file under the library root instead of
only filtering what is already loaded. A `Folder | Library` toggle sits on the search box
in each view; results appear in that view's own list, with the folder shown per row.

Matching is a case-insensitive AND over whitespace-separated terms, run against the path
relative to the root with the extension included — so `aiff` narrows by format, and
`80s cyndi` matches whichever of the two the folder happens to carry. It matches paths and
filenames, not tags: the directory walk is effectively free, while reading tags is not, and
on the libraries this was built for the title is present in the filename often enough that
an index is not worth its cost.

**A file browser that can find things**

- Sort by name or date in both browsers, with creation time and modification time as
  separate sorts.
- Quick Tag opens at the folder you configured rather than its parent.
- Each entry is `stat()`ed once instead of twice, which is measurable on a network
  filesystem.

**Delete a file from the Tag Editor**

Upstream can delete from Quick Tag, but not from the Tag Editor — the view where you
are already looking at a file closely enough to decide it should go. It now has the same
right-click item on its browser rows, plus a button beside the open file's name.

Delete also means delete. Upstream hands the file to the OS trash, which is right on a
desktop — the file lands in the Recycle Bin and the file manager can restore it. On a
server there is no desktop, so the freedesktop spec falls back to creating `.Trash-<uid>`
at the top of the volume holding the file: for a mounted library that is *inside the
library*, in a dot-directory the file browser deliberately hides. Files went somewhere
OneTagger would not show you and offered no way back. This fork removes the file, and
leaves keeping a safety net to the storage underneath, where snapshots cover overwriting
and corruption as well as deletion.

Both views confirm first, and both wait for the server to acknowledge before dropping
anything from the list — the acknowledgement carries the paths that were actually
removed, so a partial failure surfaces as an error and leaves the rest of the list
alone instead of hiding a track that is still on disk.

**File access is confined to the library**

Upstream takes the client at its word about paths. The websocket actions and the `/audio`
and `/thumb` endpoints each accept one and go straight to the filesystem, so a server
reachable by anyone reads, writes, retags and deletes anything its user can reach — a
crafted `/audio?path=/etc/passwd` needs no more than a browser.

This fork resolves every client-supplied path with `realpath` — collapsing `..`, symlinks
and redundant separators — and requires the result to sit under the `--path` the server
was started on. The resolved path is then the one used, since checking one path and
opening another is the gap the check exists to close, and the comparison is component-wise
because as text `/music` also prefixes `/musicians`. It covers listing, loading, playing,
waveforms, album art, tag writes, the autotagger and renamer inputs, playlist entries
(which are paths out of a file's contents, so they can name anything at all) and deletion.

A server started with `--expose` and no `--path` has nothing to confine access to and
refuses it outright rather than failing open. A local run with no `--path` is unrestricted,
exactly as upstream — the user driving the file dialog already has a shell.

**Linkable views**

Tag Editor and Quick Tag reflect their state — path, filter, sort, selected file, search
scope — into the URL's query string, so a view can be bookmarked, shared, and survives a
reload.

**A readable Autotagger status screen**

- The per-platform status list is also available as a sortable table, switchable at
  runtime and remembered per browser. The list stays the default.
- Failures are deduplicated when the same track fails on several platforms.

**Assorted fixes**

- A malformed tag no longer makes a track unplayable.
- Beatport search works after the site dropped `__NEXT_DATA__`; Beatsource token handling
  and query encoding are fixed.
- Duplicate custom tags are prevented and ID3 date formats standardised.
- Album art persists correctly.
- Text in the UI can be selected.


## Branches

**`main` is this fork's version of OneTagger** — upstream plus everything listed
above. Clone it, build it, and you get the additions; that is the point of the
branch, and it is what release images are built from. It is never rewound to a
plain copy of upstream.

| Branch | What it is |
|---|---|
| `main` | Upstream plus our work. The branch to build and to run. |
| `pr/<type>-<name>` | Branched from **`upstream/master`**, so its diff carries nothing of ours and a PR can be opened from it as-is. |
| `feature/<name>` | New capability, based on `main`. |
| `fix/<name>` | A correction to existing behaviour, based on `main`. |
| `docs/<name>` | Documentation, based on `main`. |

The prefix answers *can this be sent upstream?*; for `pr/` branches the next
token (`pr/fix-…`, `pr/feat-…`) answers *what is it?*. Only `pr/` makes a claim
about the base, and it is a checkable one —
`git rev-list --count upstream/master..<branch>` should be a handful of commits,
not dozens.

The distinction is capability, not intent. Plenty of the `fix/` work would be
welcome upstream; it simply cannot be *sent* from a branch whose diff against
`upstream/master` is seventy commits of unrelated work. Which it is depends on
the base, and the base is chosen before the first line is written.

Upstream changes come in by merging `upstream/master` into `main`. Nothing
upstream is removed, so `main` stays a superset rather than a divergence.

A `pr/` branch is **not** an alternative to shipping a change here: it is
branched from `upstream/master` and then merged into `main` like anything else,
so the one branch is both a ready-to-send PR and part of this fork's build. That
only works if the base is chosen *before* the change is written — start it on
`main` and it will grow to depend on whatever else is already there, after which
it cannot be lifted out without being rewritten.

One caveat on naming a branch `pr/`: generally useful is not the same as
contributable. A change that reads state our own features introduced will not
apply over there, however clean it looks here — contributing it means writing a
reduced variant, not pushing the branch.


## Installing

You can download latest binaries from [releases](https://github.com/Marekkon5/onetagger/releases)


## Docker

OneTagger can also run as a headless web service in a container. The image builds
`onetagger-cli` only (no GUI / webkit / gtk dependencies) and serves the embedded web UI
over plain HTTP on port `36913`.

The runtime is built on the [LinuxServer.io](https://www.linuxserver.io/) s6-overlay base
image, so it follows the familiar `PUID` / `PGID` / `UMASK` convention: the server runs as
a non-root user whose uid/gid you choose, and the files it creates use the umask you set —
no `--user` juggling or umask wrappers needed.

### Build the image

From a checkout of this repository:

```
docker compose build
```

Or without compose:

```
docker build -t onetagger-local .
```

Multi-stage build: `rust:1-bookworm` compiles the Vue client (pnpm) and `onetagger-cli`,
then the LinuxServer `baseimage-debian:bookworm` is the runtime with `libasound2`,
`libssl3`, and `ca-certificates`. Final image is around 295 MB.

### Run with docker compose

A `docker-compose.yml` is provided at the repo root. Set `PUID` / `PGID` to your user and
adjust the volume paths to your music library, then:

```
docker compose up -d
docker compose logs -f
```

### Run with docker run

Equivalent invocation without compose:

```
docker run -d \
  --name onetagger \
  --restart unless-stopped \
  -e PUID=1000 \
  -e PGID=1000 \
  -e UMASK=002 \
  -e TZ=Etc/UTC \
  -p 127.0.0.1:36913:36913 \
  -v "$(pwd)/config:/config" \
  -v /path/to/your/music:/music \
  --security-opt no-new-privileges:true \
  onetagger-local:latest
```

### Options reference

#### Environment variables

| Variable | Effect |
|---|---|
| `PUID` / `PGID` | Host uid/gid the server runs as, so tag writes keep correct ownership on the mounted library. Default `1000` / `1000`. |
| `UMASK` | umask for files OneTagger *creates* (cover art, brand-new files). `002` makes them group-writable (`0660`); `022` → `0644`. Default `022`. (Tag writes to *existing* files are in-place and keep the file's current mode regardless.) |
| `ONETAGGER_PATH` | Initial path shown in the UI's folder picker. Default `/music`. |
| `TZ` | Container timezone, e.g. `Europe/Oslo`. |

#### Port

The server listens on TCP `36913` inside the container. The port is a compile-time
constant (`onetagger-shared::PORT`) — to expose it on a different host port just remap,
e.g. `-p 127.0.0.1:8080:36913`.

#### Volumes

| Container path | Purpose |
|---|---|
| `/config` | OneTagger config: Spotify/Discogs OAuth tokens, custom platform settings, autotagger profiles. The server sets `XDG_CONFIG_HOME=/config`, so files live under `/config/onetagger`. Persist this across rebuilds. |
| `/music` | Music library. OneTagger writes tags in place, so anything mounted read-write here will be modified. Mount read-only (append `:ro`) for browse-only paths; a common pattern is a writable "staging" path plus a read-only "main" path. |
| `/custom-cont-init.d` | Optional. Scripts dropped here run during init (as root, *before* the drop to `PUID`/`PGID`) — e.g. to add the runtime user to extra host groups. Standard LinuxServer mechanism. |

#### Server options

The image starts `onetagger-cli server --expose --path "$ONETAGGER_PATH"`. `--expose`
binds `0.0.0.0:36913` (without it the server binds `127.0.0.1` inside the container and
the host can't reach it); the initial folder is set via `ONETAGGER_PATH`.

The other `onetagger-cli` subcommands (`autotagger`, `audiofeatures`, `renamer`,
`authorize-spotify`) can be run against the same image by overriding the entrypoint:

```
docker run --rm \
  -e PUID=1000 -e PGID=1000 \
  -v /path/to/music:/music \
  --entrypoint onetagger-cli \
  onetagger-local:latest autotagger --path /music
```

#### Hardening

The LinuxServer init starts as root, creates the `PUID`/`PGID` user, then drops privileges
to it via s6. Because of that root→user drop the container needs the default Linux
capabilities, so — unlike a plain image — **don't** `--cap-drop ALL` it.
`--security-opt no-new-privileges:true` is safe and recommended: it blocks privilege
*escalation* without preventing the init's privilege *drop*. The provided
`docker-compose.yml` sets it by default.

#### Behind a reverse proxy

The container speaks plain HTTP. To put it behind HTTPS, terminate TLS at a reverse proxy
(Caddy, Nginx, Traefik, etc.) and forward to `127.0.0.1:36913`. WebSocket upgrades on
`/ws` must be passed through. Note the upstream web client derives its WebSocket URL from
the page protocol; if your build serves the UI over HTTPS, make sure the client uses
`wss://` (see the companion client change if connecting fails over TLS).

Example Nginx fragment:

```
location / {
    proxy_pass http://127.0.0.1:36913;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

## Credits
Bas Curtiz - UI, Idea, Help  
SongRec (Shazam support) - https://github.com/marin-m/SongRec

## Support
You can support this project by donating on [PayPal](https://paypal.me/marekkon5) or [Patreon](https://www.patreon.com/onetagger)

## Compilling

### Linux & Mac
Install dependencies: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/package-manager/), [pnpm](https://pnpm.io/installation)

**Install remaining dependencies**
```
sudo apt install -y lld autogen libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.1-dev
```

**Compile UI**
```
cd client
pnpm i
pnpm run build
cd ..
```

**Compile**
```
cargo build --release
```
Output will be in: `target/release/onetagger`


### Windows
You need to install dependencies: [rustup](https://rustup.rs), [nodejs](https://nodejs.org/en/download/), [Visual Studio 2019 Build Tools](https://aka.ms/vs/16/release/vs_buildtools.exe), [pnpm](https://pnpm.io/installation)

**Compile UI:**
```
cd client
pnpm i
pnpm run build
cd ..
```

**Compile OneTagger:**
```
cargo build --release
```

Output will be inside `target\release` folder.
