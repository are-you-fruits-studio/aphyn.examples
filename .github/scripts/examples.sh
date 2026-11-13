#!/usr/bin/env bash
# Runs a command over every example in examples/<name>/scripts.
#
#   examples.sh list         names of all examples
#   examples.sh check-tags   fail unless every example pins the same tag of the official aphyn repo
#   examples.sh lint         cargo fmt --check + cargo clippy -D warnings
#   examples.sh build        cargo build
#   examples.sh use-dev      point every example at aphyn's dev branch (CI only, not committed)
#
# Set LOCKED=0 to drop --locked (needed after use-dev).

set -euo pipefail

cd "$(dirname "$0")/../.."

locked=()
if [ "${LOCKED:-1}" = "1" ]; then
    locked=(--locked)
fi

manifests() {
    shopt -s nullglob
    for manifest in examples/*/scripts/Cargo.toml; do
        echo "$manifest"
    done
}

engine_line() {
    grep -E '^aphyn = ' "$1" || true
}

case "${1:-}" in
    list)
        for manifest in $(manifests); do
            basename "$(dirname "$(dirname "$manifest")")"
        done
        ;;
    check-tags)
        tags=""
        for manifest in $(manifests); do
            if ! engine_line "$manifest" | grep -qF 'git = "https://github.com/are-you-fruits-studio/aphyn"'; then
                echo "::error file=$manifest::aphyn must come from https://github.com/are-you-fruits-studio/aphyn" >&2
                exit 1
            fi
            tag=$(engine_line "$manifest" | sed -n 's/.*tag = "\([^"]*\)".*/\1/p')
            if [ -z "$tag" ]; then
                echo "::error file=$manifest::no 'aphyn = { git = ..., tag = \"...\" }' dependency" >&2
                exit 1
            fi
            echo "$manifest: $tag"
            tags=$(printf '%s\n%s' "$tags" "$tag")
        done
        count=$(printf '%s\n' "$tags" | sed '/^$/d' | sort -u | wc -l)
        if [ "$count" -gt 1 ]; then
            echo "::error::examples pin different aphyn tags" >&2
            exit 1
        fi
        ;;
    lint)
        for manifest in $(manifests); do
            echo "::group::lint $manifest"
            cargo fmt --check --manifest-path "$manifest"
            cargo clippy "${locked[@]}" --manifest-path "$manifest" -- -D warnings
            echo "::endgroup::"
        done
        ;;
    build)
        for manifest in $(manifests); do
            echo "::group::build $manifest"
            cargo build "${locked[@]}" --manifest-path "$manifest"
            echo "::endgroup::"
        done
        ;;
    use-dev)
        for manifest in $(manifests); do
            sed -i.bak -E '/^aphyn = /s/tag = "[^"]*"/branch = "dev"/' "$manifest"
            rm "$manifest.bak"
            engine_line "$manifest"
        done
        ;;
    *)
        sed -n '2,10p' "$0" >&2
        exit 2
        ;;
esac
