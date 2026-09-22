#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

find_franking() {
    local repo_release="$SCRIPT_DIR/target/release/franking"
    local repo_debug="$SCRIPT_DIR/target/debug/franking"
    local installed_sibling="$SCRIPT_DIR/../bin/franking"
    local installed_home="$HOME/.local/share/solverforge/bin/franking"

    if [ -x "$repo_release" ]; then
        printf "%s\n" "$repo_release"
    elif [ -x "$repo_debug" ]; then
        printf "%s\n" "$repo_debug"
    elif [ -x "$installed_sibling" ]; then
        printf "%s\n" "$installed_sibling"
    elif [ -x "$installed_home" ]; then
        printf "%s\n" "$installed_home"
    elif command -v franking >/dev/null 2>&1; then
        command -v franking
    else
        printf "\n"
    fi
}

FRANKING_BIN="$(find_franking)"

if [ -z "$FRANKING_BIN" ]; then
    echo "Franking binary not found."
    echo "Build it first with 'cargo build --release' or install it."
    exit 1
fi

exec "$FRANKING_BIN" --setup "$@"
