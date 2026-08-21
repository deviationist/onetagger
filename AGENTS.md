# How we work on this fork

Conventions for this repository. Read this before branching, building or
deploying. It exists because a build that silently omits finished work is
expensive to notice and embarrassing to explain — everything below is aimed at
making that impossible rather than unlikely.

## The repo

Fork of [`Marekkon5/onetagger`](https://github.com/Marekkon5/onetagger).

| Remote | Points at | Used for |
|---|---|---|
| `origin` | `deviationist/onetagger` | our fork — all our branches live here |
| `upstream` | `Marekkon5/onetagger` | upstream; base for contributable branches |

## Branch model

- **`main` is the build branch.** It is the *integration* branch: everything we
  run is consolidated here first. Its history is ours, not upstream's.
- **`feature/<name>`** — our own work, not intended for upstream (or not yet).
- **`pr/<name>`** — work meant to be contributed. Branch these from
  **`upstream/master`**, not from `main`, so the diff stays clean and the PR
  does not drag our unrelated commits along.

Rules:

1. **Never commit a feature directly to `main`.** Branch, then merge.
2. **Never build from a feature branch.** Consolidate into `main` first —
   otherwise the image has one feature and lacks the others, which is exactly
   the regression this file exists to prevent.
3. Merge into `main` with `--no-ff` so the branch stays legible in history.
   A fast-forward is fine when `main` has not moved.
4. Delete nothing. Old feature branches are cheap; a lost branch is not.

## Keeping current with upstream

`main` is ours *on top of* upstream, never a divergence from it. Upstream work
comes in by merge:

```sh
git fetch upstream
git log --oneline main..upstream/master     # what is new over there
git checkout main
git merge upstream/master                   # resolve, then rebuild + verify
```

**Merge, do not rebase `main`.** It is pushed to `origin` and is the branch the
deployed image is built from; rebasing rewrites published history and detaches
every image tag from the commit it claims to come from.

`pr/*` branches are the exception — they are not published history in the same
sense, and they *should* be rebased onto `upstream/master` so a PR diff stays
clean.

After any upstream merge, treat it as a normal release: rebuild, redeploy and
verify. An upstream change can silently alter behaviour our features depend on,
and the merge succeeding says nothing about that.

## Build and deploy

Builds and deploys run **on quim**. Neither quim nor xavi has a host Rust
toolchain — the toolchain comes from the `rust:1-bookworm` builder stage in the
`Dockerfile`, so only Docker is required.

```sh
cd ~/code/onetagger

# 0. Preconditions -- all three must hold
git branch --show-current            # must be main
git status --porcelain               # must be empty
pgrep -f 'docker build' || true      # must be empty: see "no edits mid-build"

# 1. Confirm main really contains every feature you expect to ship
git log --oneline upstream/master..main | head -20

# 2. Type-check the client before paying for a full image build.
#    pnpm 8 -- the Dockerfile pins it, and pnpm 11 fails here on ignored
#    build scripts.
cd client && npx -y pnpm@8 run build && cd ..

# 3. Build (~10 min; the Dockerfile has no cargo cache mounts, so every
#    build recompiles the workspace from scratch)
docker build -t onetagger-local .

# 4. Tag with BOTH the moving tag and the commit it was built from
SHA=$(git rev-parse --short HEAD)
docker tag onetagger-local:latest registry.ichiva.no/onetagger:homelab
docker tag onetagger-local:latest "registry.ichiva.no/onetagger:$SHA"
docker push registry.ichiva.no/onetagger:homelab
docker push "registry.ichiva.no/onetagger:$SHA"

# 5. Deploy. onetagger-daemon shares the image -- both stacks need it.
cd ~/docker-root/onetagger        && docker compose up -d
cd ~/docker-root/onetagger-daemon && docker compose up -d
```

### No edits mid-build

The `Dockerfile` does `COPY . .` early, so a build snapshots the source when it
starts. Editing a file while a build runs produces an image that silently lacks
that edit. If the source changes, **cancel and rebuild** — the `COPY` layer
invalidates anyway, so nothing is saved by letting it finish.

### Image tags must not drift from git

`:homelab` is what `docker-compose.yml` references, so it decides what a
`docker compose up -d` actually runs — including an unrelated one months later.
Always push the `:<sha>` tag beside it: it is the rollback point, and it is the
only thing that ties a running container back to a commit.

This has already gone wrong once: `:homelab` sat ~8 hours behind a locally-built
image that was actually running, so any `up -d` would have silently rolled the
service back. (`docker compose restart`, which `nfs-compose-recover` uses, reuses
the existing container and does not have this effect.)

## Verifying a deploy

Confirm the features are in the artifact — do not assume the build carried them.

```sh
# the deployed container is running the image you just pushed
docker inspect onetagger --format '{{.Image}}'

# a distinctive string from each feature is present in the binary
docker exec onetagger grep -c 'Library results' /usr/local/bin/onetagger-cli
```

**Check that your verification tool exists before believing a zero.** The
LinuxServer runtime image has no `strings`, `find` or most of coreutils' extras;
`strings | grep -c` there returns nothing and reads as "feature missing" when it
means "command not found". A direct `grep -c` on the binary works. A zero from a
tool you have not confirmed exists is not evidence.

The Vue client is embedded in the binary via `include_dir!`, and views are lazy
chunks, so a string may live in `assets/QuickTag-*.js` rather than the main
bundle. To check what a browser actually receives:

```sh
IDX=$(curl -s http://127.0.0.1:36913/ | grep -oE 'assets/index-[A-Za-z0-9_-]+\.js' | head -1)
curl -s "http://127.0.0.1:36913/$IDX" | grep -oE '(QuickTag|TagEditor|AutotaggerStatus)-[A-Za-z0-9_-]+\.js' | sort -u
```

## Contributing upstream

- `pr/*` branches are the contributable ones; keep them based on
  `upstream/master`.
- **Open every upstream PR as a draft** (`gh pr create --draft`) and leave it to
  the maintainer's attention only on an explicit say-so.
- Before proposing a `pr/*` branch, check whether `main` carries later fixes to
  the same code that the branch lacks — proposing a superseded version wastes a
  reviewer's time.

## Where state lives

- **Platform credentials** (Discogs token, etc.) are in
  `~/docker-root/onetagger/data/config/onetagger/{settings,auto-tag}.json` on
  quim. They are outside git by design, so a git-based restore does **not**
  bring them back. Back them up separately.
- **`client/dist`** is generated. `onetagger-ui` embeds it at compile time via
  `include_dir!`, so the crate cannot build without it — which is why step 2
  above is also a prerequisite for any local `cargo` work.

## Conventions

- Commit messages explain **why**, in prose, not what the diff already shows.
- Persist UI state in the right place: the **query string** for shareable view
  state (path, filter, sort, selection — a link should reproduce what the sender
  saw), **`localStorage`** for per-device display preferences, and the
  server-side settings file for genuine cross-device configuration.
