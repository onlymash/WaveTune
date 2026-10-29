#!/bin/sh
# WaveTune - Unofficial multi-brand wireless headset control center for Linux
# Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, version 3.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https:#www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-only

# Install WaveTune's runtime files into a staging directory.
#
# Shared by the Arch PKGBUILDs and the Debian packaging so the installed file
# layout is defined in exactly one place.
#
# Usage: packaging/install.sh <destdir> [binary]
#   destdir  staging root, e.g. "$pkgdir" (no trailing slash needed)
#   binary   path to the built executable (default: target/release/wavetune)
#
# Environment:
#   PREFIX   installation prefix inside destdir (default: /usr)
set -eu

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "usage: $0 <destdir> [binary]" >&2
    exit 2
fi

destdir=$1
binary=${2:-target/release/wavetune}
prefix=${PREFIX:-/usr}
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app=io.github.onlymash.WaveTune

if [ ! -f "$binary" ]; then
    echo "error: binary not found: $binary" >&2
    exit 1
fi

# Binary
install -Dm755 "$binary" "$destdir$prefix/bin/wavetune"

# Desktop launcher and AppStream metainfo
install -Dm644 "$root/data/$app.desktop" \
    "$destdir$prefix/share/applications/$app.desktop"
install -Dm644 "$root/data/$app.metainfo.xml" \
    "$destdir$prefix/share/metainfo/$app.metainfo.xml"

# Application icons
install -Dm644 "$root/assets/icons/$app.svg" \
    "$destdir$prefix/share/icons/hicolor/scalable/apps/$app.svg"
install -Dm644 "$root/assets/icons/$app-circle.svg" \
    "$destdir$prefix/share/icons/hicolor/scalable/apps/$app-circle.svg"
install -Dm644 "$root/assets/icons/$app-tws.svg" \
    "$destdir$prefix/share/icons/hicolor/scalable/apps/$app-tws.svg"
install -Dm644 "$root/assets/icons/$app-tws-circle.svg" \
    "$destdir$prefix/share/icons/hicolor/scalable/apps/$app-tws-circle.svg"
install -Dm644 "$root/assets/icons/$app-symbolic.svg" \
    "$destdir$prefix/share/icons/hicolor/symbolic/apps/$app-symbolic.svg"
