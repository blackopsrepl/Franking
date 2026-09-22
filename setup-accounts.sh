#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

find_franking() {
    local repo_release="$SCRIPT_DIR/target/release/franking"
    local repo_debug="$SCRIPT_DIR/target/debug/franking"
    local installed_cargo="$HOME/.cargo/bin/franking"
    local installed_wrapper="$SCRIPT_DIR/../bin/franking"

    if [ -x "$repo_release" ]; then
        printf "%s\n" "$repo_release"
    elif [ -x "$repo_debug" ]; then
        printf "%s\n" "$repo_debug"
    elif [ -x "$installed_cargo" ]; then
        printf "%s\n" "$installed_cargo"
    elif [ -x "$installed_wrapper" ]; then
        printf "%s\n" "$installed_wrapper"
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
