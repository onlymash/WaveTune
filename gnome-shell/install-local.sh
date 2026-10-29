#!/usr/bin/env bash
# WaveTune - Unofficial multi-brand wireless headset control center for Linux
# SPDX-License-Identifier: GPL-3.0-only
#
# Install or symlink the WaveTune GNOME Shell Quick Settings extension into
# ~/.local/share/gnome-shell/extensions/ for local development and testing.
#
# Usage:
#   ./gnome-shell/install-local.sh [--copy | --uninstall]

set -euo pipefail

UUID="wavetune@onlymash.github.com"
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
SRC_DIR="$SCRIPT_DIR/$UUID"
EXT_BASE="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions"
DEST_DIR="$EXT_BASE/$UUID"

mode="symlink"
case "${1:-}" in
    "" | --symlink)
        mode="symlink"
        ;;
    --copy)
        mode="copy"
        ;;
    --uninstall | --remove)
        mode="uninstall"
        ;;
    -h | --help)
        echo "Usage: $0 [--symlink | --copy | --uninstall]"
        exit 0
        ;;
    *)
        echo "Unknown option: $1" >&2
        exit 2
        ;;
esac

if [[ $mode == "uninstall" ]]; then
    if command -v gnome-extensions >/dev/null 2>&1; then
        gnome-extensions disable "$UUID" 2>/dev/null || true
    fi
    rm -rf "$DEST_DIR"
    echo "Removed $DEST_DIR"
    exit 0
fi

mkdir -p "$EXT_BASE"
rm -rf "$DEST_DIR"

if [[ $mode == "symlink" ]]; then
    ln -s "$SRC_DIR" "$DEST_DIR"
    echo "Symlinked $DEST_DIR -> $SRC_DIR"
else
    mkdir -p "$DEST_DIR"
    install -Dm644 "$SRC_DIR/metadata.json" "$DEST_DIR/metadata.json"
    install -Dm644 "$SRC_DIR/extension.js" "$DEST_DIR/extension.js"
    echo "Copied extension files to $DEST_DIR"
fi

if command -v gnome-extensions >/dev/null 2>&1; then
    if gnome-extensions info "$UUID" >/dev/null 2>&1; then
        gnome-extensions enable "$UUID" && echo "Enabled $UUID"
    else
        echo "Note: GNOME Shell has not indexed $UUID yet. Log out and back in once, then run:"
        echo "  gnome-extensions enable $UUID"
    fi
fi

