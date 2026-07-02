#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS] <version>

Bump version in all project manifests, commit, create a git tag, and push.

Arguments:
    version         Version number (e.g. 0.0.23 or v0.0.23)

Options:
    -m, --message   Tag message (default: "Release <version>")
    -r, --remote    Remote name (default: origin)
    -d, --delete    Delete the tag locally and remotely
    -n, --dry-run   Show what would be done without executing
    --no-bump       Skip version bump (only create tag)
    -h, --help      Show this help message

Files updated during version bump:
    package.json              (npm version)
    Cargo.toml                (workspace version)
    src-tauri/tauri.conf.json (Tauri app version)
EOF
    exit "${1:-0}"
}

VERSION=""
MESSAGE=""
REMOTE="origin"
DELETE=false
DRY_RUN=false
NO_BUMP=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -m|--message)  MESSAGE="$2"; shift 2 ;;
        -r|--remote)   REMOTE="$2"; shift 2 ;;
        -d|--delete)   DELETE=true; shift ;;
        -n|--dry-run)  DRY_RUN=true; shift ;;
        --no-bump)     NO_BUMP=true; shift ;;
        -h|--help)     usage 0 ;;
        -*)            echo "Error: unknown option '$1'" >&2; usage 1 ;;
        *)             VERSION="$1"; shift ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    echo "Error: version number is required" >&2
    usage 1
fi

cd "$PROJECT_ROOT"

if ! git rev-parse --is-inside-work-tree &>/dev/null; then
    echo "Error: not inside a git repository" >&2
    exit 1
fi

TAG="$VERSION"
[[ "$TAG" != v* ]] && TAG="v${VERSION}"

SEMVER="${TAG#v}"

if ! [[ "$SEMVER" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "Error: '$SEMVER' is not a valid semver version" >&2
    exit 1
fi

run() {
    if $DRY_RUN; then
        echo "[dry-run] $*"
    else
        "$@"
    fi
}

if $DELETE; then
    echo "Deleting tag '$TAG' ..."
    run git tag -d "$TAG" 2>/dev/null || echo "  Local tag not found, skipping"
    run git push "$REMOTE" --delete "$TAG" 2>/dev/null || echo "  Remote tag not found, skipping"
    echo "Done."
    exit 0
fi

if git rev-parse "$TAG" &>/dev/null; then
    echo "Error: tag '$TAG' already exists" >&2
    echo "  Use -d to delete it first, or choose a different version" >&2
    exit 1
fi

if [[ -n "$(git status --porcelain)" ]] && ! $NO_BUMP; then
    echo "Error: working tree is not clean. Commit or stash changes first." >&2
    git status --short >&2
    exit 1
fi

bump_version() {
    local file="$1"
    local old new

    if [[ ! -f "$file" ]]; then
        echo "  Warning: $file not found, skipping" >&2
        return
    fi

    case "$file" in
        *.json)
            old=$(grep -oP '"version"\s*:\s*"\K[^"]+' "$file" | head -1)
            ;;
        *.toml)
            old=$(grep -oP '^version\s*=\s*"\K[^"]+' "$file" | head -1)
            ;;
    esac

    if [[ -z "${old:-}" ]]; then
        echo "  Warning: could not find version in $file, skipping" >&2
        return
    fi

    new="$SEMVER"
    if [[ "$old" == "$new" ]]; then
        echo "  $file: already at $new"
        return
    fi

    echo "  $file: $old → $new"
    if ! $DRY_RUN; then
        case "$file" in
            *.json)
                sed -i "s/\"version\": \"$old\"/\"version\": \"$new\"/" "$file"
                ;;
            *.toml)
                sed -i "s/^version = \"$old\"/version = \"$new\"/" "$file"
                ;;
        esac
    fi
}

if ! $NO_BUMP; then
    echo "Bumping version to $SEMVER ..."
    bump_version "package.json"
    bump_version "Cargo.toml"
    bump_version "src-tauri/tauri.conf.json"

    if ! $DRY_RUN; then
        if git diff --quiet package.json Cargo.toml src-tauri/tauri.conf.json; then
            echo "  No version changes needed."
        else
            git add package.json Cargo.toml src-tauri/tauri.conf.json
            git commit -m "chore: bump version to $TAG"
            echo "  Version bump committed."
        fi
    fi
fi

: "${MESSAGE:=Release $TAG}"

echo "Creating tag '$TAG' on $(git log -1 --format='%h %s') ..."
run git tag -a "$TAG" -m "$MESSAGE"

echo "Pushing to '$REMOTE' ..."
run git push "$REMOTE" HEAD "$TAG"

echo ""
echo "Done. Version $TAG released."
echo "  Tag:    $TAG"
echo "  Commit: $(git log -1 --format='%h %s')"
echo "  Remote: $REMOTE"
