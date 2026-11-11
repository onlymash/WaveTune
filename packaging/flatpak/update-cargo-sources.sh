#!/usr/bin/env bash
# Regenerate cargo-sources.json from Cargo.lock.
# Run this whenever Cargo.lock changes (dependency bumps, new crates).
set -euo pipefail

here=$(cd -- "$(dirname -- "$0")" && pwd)
root=$(cd -- "$here/../.." && pwd)

# Pinned so a random upstream change can't break the output format on you.
GEN_REV=9d86d336a39b9af38b86f114b6ce5820fe260f91
GEN_URL="https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$GEN_REV/cargo/flatpak-cargo-generator.py"

cache="${XDG_CACHE_HOME:-$HOME/.cache}/wavetune-flatpak"
venv="$cache/venv"
gen="$cache/flatpak-cargo-generator-$GEN_REV.py"

mkdir -p "$cache"

if [ ! -x "$venv/bin/python" ]; then
    echo ">> creating venv in $venv"
    python3 -m venv "$venv"
    "$venv/bin/pip" install -q aiohttp tomlkit
fi

if [ ! -f "$gen" ]; then
    echo ">> downloading flatpak-cargo-generator ($GEN_REV)"
    curl -fsSL "$GEN_URL" -o "$gen"
fi

echo ">> generating cargo-sources.json"
"$venv/bin/python" "$gen" "$root/Cargo.lock" -o "$here/cargo-sources.json"

count=$("$venv/bin/python" -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$here/cargo-sources.json")
echo ">> done, $count entries"
