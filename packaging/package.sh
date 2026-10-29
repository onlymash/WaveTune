#!/usr/bin/env bash
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

# Build distributable WaveTune packages from the current working tree.
#
# Usage: packaging/package.sh [arch] [git] [deb] [all] [--no-check]
#
#   arch        Build an Arch Linux package with makepkg, using a snapshot of
#               the working tree (uncommitted edits and non-ignored new files).
#   git         Build the wavetune-git Arch package from this repository's
#               committed HEAD (uncommitted changes are not included).
#   deb         Build a Debian package with dpkg-deb.
#   all         Build arch and deb.
#   --no-check  Skip `cargo test`.
#
# With no target, builds `arch` if makepkg is available, otherwise `deb`.
# All output goes to target/dist/; intermediate files stay under target/dist/<format>/.
#
# Environment:
#   DEB_REVISION  Debian revision appended to the version (default: 1)
set -euo pipefail
umask 022

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

pkgname=wavetune
srcname=WaveTune
dist_dir="$repo_root/target/dist"
cargo_target_dir=${CARGO_TARGET_DIR:-$repo_root/target}
run_check=1
targets=()

info() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m==> WARNING:\033[0m %s\n' "$*" >&2; }
die() {
    printf '\033[1;31m==> ERROR:\033[0m %s\n' "$*" >&2
    exit 1
}
usage() { sed -n '2,/^set -euo/{/^set -euo/d;s/^# \{0,1\}//;p}' "${BASH_SOURCE[0]}"; }
require() { command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"; }

while (($#)); do
    case $1 in
        arch | git | deb) targets+=("$1") ;;
        all) targets+=(arch deb) ;;
        --no-check) run_check=0 ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            die "unknown argument: $1"
            ;;
    esac
    shift
done

if ((${#targets[@]} == 0)); then
    if command -v makepkg >/dev/null 2>&1; then
        targets=(arch)
    else
        targets=(deb)
    fi
fi

version=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -n1)
[[ -n $version ]] || die "could not read package version from Cargo.toml"

# Create a source tarball laid out like the GitHub release archive
# (WaveTune-<version>/...) from the working tree: tracked and untracked files,
# excluding anything matched by .gitignore.
make_source_tarball() {
    local out=$1 file
    require git
    require tar
    git ls-files -z --cached --others --exclude-standard |
        while IFS= read -r -d '' file; do
            # Skip tracked files deleted in the working tree.
            [[ -e $file || -L $file ]] && printf '%s\0' "$file"
        done |
        tar --null --files-from=- --transform="s,^,$srcname-$version/," -czf "$out"
}

build_arch() {
    require makepkg
    local work="$dist_dir/arch"

    info "Building Arch package $pkgname $version"
    rm -rf "$work"
    mkdir -p "$work"
    bash "$repo_root/packaging/arch/generate.sh" release --reset-sums --output "$work/PKGBUILD"
    make_source_tarball "$work/$pkgname-$version.tar.gz"

    # The local snapshot differs from the release tarball, so skip checksums.
    local args=(--force --cleanbuild --skipchecksums)
    ((run_check)) || args+=(--nocheck)
    (
        cd "$work"
        PKGDEST="$dist_dir" SRCDEST="$work" BUILDDIR="$work/build" makepkg "${args[@]}"
    )
}

build_git() {
    require makepkg
    require git
    local work="$dist_dir/arch-git"

    if [[ -n $(git status --porcelain --untracked-files=no) ]]; then
        warn "uncommitted changes are not included in wavetune-git; commit them first"
    fi

    info "Building Arch package wavetune-git from local HEAD ($(git rev-parse --short HEAD))"
    rm -rf "$work"
    mkdir -p "$work"
    # Clone from this local repository instead of GitHub so unpushed commits are tested.
    bash "$repo_root/packaging/arch/generate.sh" git \
        --source "\$_pkgname::git+file://$repo_root" \
        --output "$work/PKGBUILD"

    local args=(--force --cleanbuild)
    ((run_check)) || args+=(--nocheck)
    (
        cd "$work"
        PKGDEST="$dist_dir" SRCDEST="$work" BUILDDIR="$work/build" makepkg "${args[@]}"
    )
}

deb_arch() {
    if command -v dpkg >/dev/null 2>&1; then
        dpkg --print-architecture
        return
    fi
    case $(uname -m) in
        x86_64) echo amd64 ;;
        aarch64) echo arm64 ;;
        *) die "unsupported architecture for deb: $(uname -m)" ;;
    esac
}

build_deb() {
    require cargo
    require dpkg-deb
    local revision=${DEB_REVISION:-1} arch work root installed_size out
    arch=$(deb_arch)
    work="$dist_dir/deb"
    root="$work/root"
    out="$dist_dir/${pkgname}_${version}-${revision}_${arch}.deb"

    if [[ ! -f /etc/debian_version ]]; then
        warn "building .deb on a non-Debian host: the binary links against this host's" \
            "glibc/GTK/libadwaita and may not run on Debian/Ubuntu. Build release" \
            ".deb files on the target distribution."
    fi

    info "Building Debian package $pkgname $version-$revision ($arch)"
    if ((run_check)); then
        cargo test --locked
    fi
    cargo build --locked --release

    rm -rf "$work"
    mkdir -p "$root/DEBIAN"
    sh packaging/install.sh "$root" "$cargo_target_dir/release/$pkgname"
    install -Dm644 packaging/debian/copyright "$root/usr/share/doc/$pkgname/copyright"

    installed_size=$(du -sk --exclude=DEBIAN "$root" | cut -f1)
    sed -e "s/@VERSION@/$version-$revision/" \
        -e "s/@ARCH@/$arch/" \
        -e "s/@INSTALLED_SIZE@/$installed_size/" \
        packaging/debian/control.in >"$root/DEBIAN/control"

    mkdir -p "$dist_dir"
    dpkg-deb --root-owner-group -Zxz --build "$root" "$out"
}

for target in "${targets[@]}"; do
    "build_$target"
done

info "Packages in ${dist_dir#"$repo_root"/}:"
find "$dist_dir" -maxdepth 1 -type f \( -name '*.pkg.tar.*' -o -name '*.deb' \) -printf '    %f\n' | sort
