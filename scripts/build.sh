#!/usr/bin/env bash
#
# Build, tag, push and (optionally) deploy the OneTagger image.
#
# Everything site-specific -- registry, image name, tags, deploy paths -- comes
# from .env (gitignored; see .env.example). Nothing about anyone's
# infrastructure belongs in this file.
#
# The checks below are not ceremony. Each one corresponds to a way a build has
# actually gone wrong:
#
#   * building from a feature branch ships some features and silently drops the
#     rest, which reads as a lost feature rather than a process mistake;
#   * the Dockerfile does `COPY . .` early, so editing source while a build runs
#     produces an image quietly missing the edit;
#   * pushing only the moving tag lets it drift from the commit it claims to be,
#     so a later `docker compose up -d` can roll a service backwards.
#
# Usage:  scripts/build.sh [--no-push] [--deploy]

set -euo pipefail

cd "$(dirname "$0")/.."
REPO_ROOT="$PWD"

RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; RESET=$'\033[0m'
die()  { printf '%s\n' "${RED}error:${RESET} $*" >&2; exit 1; }
note() { printf '%s\n' "${GREEN}==>${RESET} $*"; }
warn() { printf '%s\n' "${YELLOW}warning:${RESET} $*" >&2; }

PUSH=1
DEPLOY=0
for arg in "$@"; do
    case "$arg" in
        --no-push) PUSH=0 ;;
        --deploy)  DEPLOY=1 ;;
        *) die "unknown argument: $arg" ;;
    esac
done

[ -f .env ] || die "no .env -- copy .env.example to .env and fill it in"
# shellcheck disable=SC1091
set -a; . ./.env; set +a

: "${IMAGE_NAME:?IMAGE_NAME must be set in .env}"
: "${LOCAL_TAG:=onetagger-local:latest}"
: "${MOVING_TAG:=latest}"

# --- Preconditions ---------------------------------------------------------

BRANCH="$(git branch --show-current)"
BUILD_BRANCH="${BUILD_BRANCH:-main}"
[ "$BRANCH" = "$BUILD_BRANCH" ] || die \
    "on '$BRANCH', not '$BUILD_BRANCH'. Consolidate features into $BUILD_BRANCH first; \
building from a feature branch ships an image missing the others."

[ -z "$(git status --porcelain)" ] || die \
    "working tree is dirty. Commit or stash first, so the image corresponds to a commit."

if pgrep -f "docker build .*${IMAGE_NAME}" >/dev/null 2>&1; then
    die "another build of ${IMAGE_NAME} is already running"
fi

SHA="$(git rev-parse --short HEAD)"
note "building ${IMAGE_NAME} from ${BRANCH} @ ${SHA}"

# --- Client -----------------------------------------------------------------
# onetagger-ui embeds client/dist via include_dir! at compile time, so a broken
# client fails the Rust build ten minutes in. Type-check it first -- it costs
# seconds. pnpm 8: the Dockerfile pins it, and pnpm >=11 aborts here on ignored
# build scripts.
note "type-checking the client"
( cd client && npx -y pnpm@8 run build >/dev/null ) || die "client build failed"

# --- Image ------------------------------------------------------------------
note "docker build (no cargo cache mounts: expect a full workspace compile)"
docker build -t "$LOCAL_TAG" .

if [ -z "${REGISTRY:-}" ]; then
    note "REGISTRY unset -- built ${LOCAL_TAG} locally, nothing pushed"
    exit 0
fi

REMOTE="${REGISTRY}/${IMAGE_NAME}"
docker tag "$LOCAL_TAG" "${REMOTE}:${MOVING_TAG}"
docker tag "$LOCAL_TAG" "${REMOTE}:${SHA}"
note "tagged ${REMOTE}:${MOVING_TAG} and ${REMOTE}:${SHA}"

if [ "$PUSH" -eq 1 ]; then
    # Both, always. The moving tag is what a compose file resolves; the sha tag
    # is what lets you say which commit is running, and roll back to it.
    docker push "${REMOTE}:${MOVING_TAG}"
    docker push "${REMOTE}:${SHA}"
    note "pushed both tags"
else
    warn "--no-push: tags exist locally only"
fi

# --- Deploy -----------------------------------------------------------------
if [ "$DEPLOY" -eq 1 ]; then
    [ -n "${DEPLOY_DIRS:-}" ] || die "--deploy given but DEPLOY_DIRS is empty in .env"
    for dir in $DEPLOY_DIRS; do
        [ -d "$dir" ] || die "deploy dir does not exist: $dir"
        note "deploying $dir"
        ( cd "$dir" && docker compose up -d )
    done

    # Every stack sharing the image must end up on it; one left behind is the
    # same class of bug as building from the wrong branch, just later.
    note "verifying deployed images"
    for dir in $DEPLOY_DIRS; do
        name="$(basename "$dir")"
        running="$(docker inspect "$name" --format '{{.Image}}' 2>/dev/null || echo unknown)"
        expected="$(docker image inspect "${REMOTE}:${SHA}" --format '{{.Id}}' 2>/dev/null || echo unknown)"
        if [ "$running" = "$expected" ]; then
            printf '    %-24s ok\n' "$name"
        else
            warn "$name is running $running, expected $expected"
        fi
    done
fi

note "done: ${SHA}"
