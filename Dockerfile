# syntax=docker/dockerfile:1.6

# ---- Build stage ----------------------------------------------------------
FROM rust:1-bookworm AS builder

ARG DEBIAN_FRONTEND=noninteractive

# Build deps:
# - libasound2-dev: onetagger-player links alsa even when the binary runs in server mode
# - lld: matches .cargo/config.toml's `link-arg=-fuse-ld=lld` for x86_64-unknown-linux-gnu
# - nodejs/npm + pnpm: build the Vue/Quasar client (client/dist) which onetagger-ui embeds via include_dir!
# We skip libwebkit2gtk-4.1-dev: only the `onetagger` GUI crate needs it, and we build onetagger-cli only.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        libasound2-dev \
        ca-certificates \
        git \
        lld \
        nodejs \
        npm \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g pnpm@8

WORKDIR /src

# Source comes from this repo (Dockerfile lives at the repo root, alongside Cargo.toml, crates/, client/).
# .dockerignore filters target/, node_modules/, dist/, .git/ so local edits are picked up without bloating context.
COPY . .

# Client must be built before cargo: onetagger-ui's include_dir!("../../client/dist") runs at compile time.
# client/.gitignore excludes pnpm-lock.yaml, so checkouts never have one — `pnpm install` (not `--frozen-lockfile`).
RUN cd client && pnpm install && pnpm run build

# -p onetagger-cli skips the GUI binary (webkit/gtk transitive deps).
RUN cargo build --release -p onetagger-cli


# ---- Runtime stage (LinuxServer.io s6-overlay base) -----------------------
# Rebased onto the LSIO baseimage so the operator controls identity + umask the
# standard way (PUID/PGID/UMASK env), instead of a `user:`/`umask` wrapper in
# compose. The base brings the `abc` user, the root->abc privilege drop, UMASK
# handling, and /custom-cont-init.d support. bookworm matches the builder's glibc.
FROM ghcr.io/linuxserver/baseimage-debian:bookworm AS runtime

ARG DEBIAN_FRONTEND=noninteractive

# Runtime libs the onetagger-cli binary dynamically links:
# - libasound2: alsa link from onetagger-player
# - libssl3: openssl runtime for HTTPS calls to MusicBrainz/Discogs/Spotify/Beatport/...
# - ca-certificates: trust store for those outbound calls
RUN apt-get update && apt-get install -y --no-install-recommends \
        libasound2 \
        libssl3 \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/onetagger-cli /usr/local/bin/onetagger-cli

# s6-overlay v3 service tree: see root/etc/s6-overlay/s6-rc.d/svc-onetagger.
# Runs onetagger-cli as `abc` after the base finishes PUID/PGID/UMASK setup.
COPY root/ /
RUN chmod +x /etc/s6-overlay/s6-rc.d/svc-onetagger/run

# Server binds 0.0.0.0:36913 with --expose (set in the s6 run script).
EXPOSE 36913

# No ENTRYPOINT/CMD/USER here: the LSIO base's /init is PID 1 and supervises the
# svc-onetagger service. Identity = PUID/PGID, umask = UMASK, both from env.
