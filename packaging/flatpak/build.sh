#!/usr/bin/env bash
# Build + install the Flatpak from the current checkout.
# Regenerates cargo-sources.json first if Cargo.lock is newer.
#
#   ./build.sh          build and install for the current user
#   ./build.sh --run    same, then launch it
set -euo pipefail

here=$(cd -- "$(dirname -- "$0")" && pwd)
root=$(cd -- "$here/../.." && pwd)
app=io.github.onlymash.WaveTune

if [ ! -f "$here/cargo-sources.json" ] || [ "$root/Cargo.lock" -nt "$here/cargo-sources.json" ]; then
    "$here/update-cargo-sources.sh"
fi

cd "$here"
flatpak-builder --user --install --force-clean \
    --install-deps-from=flathub \
    build-dir "$app.yml"

if [ "${1:-}" = "--run" ]; then
    flatpak run "$app"
fi
