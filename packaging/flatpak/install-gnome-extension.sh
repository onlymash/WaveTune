#!/usr/bin/env bash
# Link the GNOME Shell Quick Settings extension out of the installed Flatpak.
# Flatpak can't export shell extensions itself, so this symlinks it into
# ~/.local/share/gnome-shell/extensions. The link targets flatpak's "active"
# deployment, so it follows app updates automatically.
#
#   ./install-gnome-extension.sh              link + enable
#   ./install-gnome-extension.sh --uninstall  disable + remove
set -euo pipefail

app=io.github.onlymash.WaveTune
uuid=wavetune@onlymash.github.com
dest="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$uuid"

if [ "${1:-}" = "--uninstall" ]; then
    gnome-extensions disable "$uuid" 2>/dev/null || true
    rm -rf "$dest"
    echo "Removed $dest"
    exit 0
fi

# e.g. ~/.local/share/flatpak/app/<id>/x86_64/master/<hash> -> use the sibling "active" link
loc=$(flatpak info --show-location "$app" 2>/dev/null) || {
    echo "error: $app is not installed" >&2
    exit 1
}
src="$(dirname "$loc")/active/files/share/gnome-shell/extensions/$uuid"

if [ ! -f "$src/metadata.json" ]; then
    echo "error: extension not found in the flatpak at $src" >&2
    exit 1
fi

mkdir -p "$(dirname "$dest")"
rm -rf "$dest"
ln -s "$src" "$dest"
echo "Linked $dest -> $src"

if gnome-extensions enable "$uuid" 2>/dev/null; then
    echo "Enabled. If it doesn't show up yet, log out and back in (Wayland can't reload the shell)."
else
    echo "Log out and back in, then enable it with: gnome-extensions enable $uuid"
fi
