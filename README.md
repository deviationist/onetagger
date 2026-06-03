<p align='center'>
    <img alt='Logo' src='https://raw.githubusercontent.com/Marekkon5/onetagger/master/assets/onetagger-logo-github.png'>
</p>
<h1 align='center'>The ultimate cross-platform tagger for DJs</h1>

<h3 align='center'><b>
<a href='https://onetagger.github.io/'>Website</a> | <a href='https://github.com/Marekkon5/onetagger/releases/'>Latest Release</a>
</b></h3>
<br>

<p align='center'>
    <img alt='Version Badge' src='https://img.shields.io/github/v/release/marekkon5/onetagger?label=Latest%20Release'>
    <img alt='Supported OS' src='https://img.shields.io/badge/OS-Windows%2C%20Mac%20OS%2C%20Linux-orange'>
    <img alt='Build Status' src='https://img.shields.io/github/actions/workflow/status/marekkon5/onetagger/build.yml?branch=master'>
</p>

<h3 align='center'><b></b></h3>
<hr>

Cross-platform music tagger.
It can fetch metadata from Beatport, Traxsource, Juno Download, Discogs, Musicbrainz and Spotify.
It is also able to fetch Spotify's Audio Features based on ISRC & exact match. 
There is a manual tag editor and quick tag editor which lets you use keyboard shortcuts. Written in Rust, Vue.js and Quasar.

MP3, AIFF, FLAC, M4A (AAC, ALAC) supported.

*For more info and tutorials check out our [website](https://onetagger.github.io/).*

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4


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
