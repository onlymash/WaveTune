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
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-only

# Generate Arch Linux PKGBUILDs and standalone AUR Git repositories for WaveTune.
#
# Usage:
#   packaging/arch/generate.sh [release|git|all] [options]
#
# Targets:
#   release, wavetune      Generate non-git release AUR repo in packaging/arch/wavetune/
#   git, wavetune-git      Generate VCS (-git) AUR repo in packaging/arch/wavetune-git/
#   all                    Generate both wavetune and wavetune-git (default)
#
# Options:
#   -o, --output <file>    Write only the generated PKGBUILD to <file> (requires single target)
#   --stdout               Print only the generated PKGBUILD to stdout (requires single target)
#   --source <spec>        Override source=() entry (e.g. "wavetune::git+file:///path/to/repo")
#   --pkgrel <n>           Set pkgrel (default: 1)
#   --sha256 <sum>         Set sha256sums for the release package
#   --reset-sums           Reset release sha256sums to 'SKIP' even if already pinned
#   --updpkgsums           Run `updpkgsums` on the release PKGBUILD before generating .SRCINFO
#   --no-git-init          Do not initialize a Git repository inside the target AUR directory
#   --check                Validate generated AUR repositories (PKGBUILD, .SRCINFO, Git rules)
#   -h, --help             Show this help message
set -euo pipefail
umask 022

arch_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$arch_dir/../.." && pwd)

pkgname_base="wavetune"
src_repo_name="WaveTune"
maintainer="onlymash <fiepi.dev@gmail.com>"
repo_url="https://github.com/onlymash/WaveTune"
pkgdesc_base="Unofficial wireless headset control center for Linux"

pkgrel="1"
output_file=""
stdout_only=0
source_override=""
sha256_override=""
reset_sums=0
run_updpkgsums=0
git_init=1
check_only=0
targets=()

info() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m==> WARNING:\033[0m %s\n' "$*" >&2; }
die() {
    printf '\033[1;31m==> ERROR:\033[0m %s\n' "$*" >&2
    exit 1
}
usage() { sed -n '20,/^set -euo/{/^set -euo/d;s/^# \{0,1\}//;p}' "${BASH_SOURCE[0]}"; }
require() { command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"; }

while (($#)); do
    case $1 in
        release | wavetune | arch)
            targets+=(release)
            ;;
        git | wavetune-git)
            targets+=(git)
            ;;
        all)
            targets+=(release git)
            ;;
        -o | --output)
            (($# >= 2)) || die "missing argument for $1"
            output_file=$2
            shift
            ;;
        --stdout)
            stdout_only=1
            ;;
        --source)
            (($# >= 2)) || die "missing argument for $1"
            source_override=$2
            shift
            ;;
        --pkgrel)
            (($# >= 2)) || die "missing argument for $1"
            pkgrel=$2
            shift
            ;;
        --sha256)
            (($# >= 2)) || die "missing argument for $1"
            sha256_override=$2
            shift
            ;;
        --reset-sums)
            reset_sums=1
            ;;
        --updpkgsums)
            run_updpkgsums=1
            ;;
        --no-git-init)
            git_init=0
            ;;
        --check)
            check_only=1
            ;;
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
    targets=(release git)
fi

# Deduplicate targets while preserving order.
dedup_targets=()
for t in "${targets[@]}"; do
    seen=0
    for existing in "${dedup_targets[@]}"; do
        [[ $existing == "$t" ]] && seen=1 && break
    done
    ((seen)) || dedup_targets+=("$t")
done
targets=("${dedup_targets[@]}")

if ((stdout_only)) && [[ -n $output_file ]]; then
    die "--stdout and --output cannot be used together"
fi

if ((stdout_only)) || [[ -n $output_file ]]; then
    ((${#targets[@]} == 1)) || die "--stdout / --output requires a single target (release or git)"
fi

cargo_version() {
    local ver
    ver=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)
    [[ -n $ver ]] || die "could not read package version from $repo_root/Cargo.toml"
    printf '%s\n' "$ver"
}

cargo_license() {
    local lic
    lic=$(sed -n 's/^license[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)
    [[ -n $lic ]] || die "could not read license from $repo_root/Cargo.toml"
    printf '%s\n' "$lic"
}

compute_git_pkgver() {
    require git
    (
        cd "$repo_root"
        if git describe --long --tags --match 'v[0-9]*' >/dev/null 2>&1; then
            git describe --long --tags --match 'v[0-9]*' --abbrev=7 |
                sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
        else
            printf 'r%s.g%s\n' "$(git rev-list --count HEAD)" "$(git rev-parse --short=7 HEAD)"
        fi
    )
}

resolve_release_sha256() {
    local target_ver=$1 existing_pkgbuild="$arch_dir/$pkgname_base/PKGBUILD"
    if [[ -n $sha256_override ]]; then
        printf '%s\n' "$sha256_override"
        return
    fi
    if ((!reset_sums)) && [[ -f $existing_pkgbuild ]]; then
        local existing_ver existing_sum
        existing_ver=$(sed -n 's/^pkgver=//p' "$existing_pkgbuild" | head -n1)
        existing_sum=$(sed -n "s/^sha256sums=('\\([^']*\\)')/\\1/p" "$existing_pkgbuild" | head -n1)
        if [[ $existing_ver == "$target_ver" && -n $existing_sum && $existing_sum != "SKIP" ]]; then
            printf '%s\n' "$existing_sum"
            return
        fi
    fi
    printf 'SKIP\n'
}

render_common_depends() {
    cat <<'EOF'
depends=(
    'bluez'
    'bluez-libs'
    'dbus'
    'glib2'
    'glibc'
    'gtk4'
    'hicolor-icon-theme'
    'libadwaita'
    'libgcc'
    'pango'
)
optdepends=(
    'gnome-shell: GNOME Quick Settings panel integration'
    'gnome-shell-extension-appindicator: system tray icon support on GNOME'
)
EOF
}

render_common_build_functions() {
    local srcdir_var=$1
    cat <<EOF
prepare() {
    cd "$srcdir_var"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "\$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$srcdir_var"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

check() {
    cd "$srcdir_var"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    export XDG_CACHE_HOME="\$srcdir/.cache"
    export XDG_CONFIG_HOME="\$srcdir/.config"
    export XDG_DATA_HOME="\$srcdir/.local/share"
    # Run headless unit/protocol tests, skipping interactive UI tests that require a display server
    cargo test --frozen --all-features -- --skip ui::
}

package() {
    cd "$srcdir_var"
    sh packaging/install.sh "\$pkgdir" target/release/$pkgname_base
}
EOF
}

render_release_pkgbuild() {
    local ver lic sum src_entry
    ver=$(cargo_version)
    lic=$(cargo_license)
    sum=$(resolve_release_sha256 "$ver")
    if [[ -n $source_override ]]; then
        src_entry=$source_override
    else
        src_entry="\$pkgname-\$pkgver.tar.gz::\$url/archive/refs/tags/v\$pkgver.tar.gz"
    fi

    cat <<EOF
# Maintainer: $maintainer
pkgname=$pkgname_base
pkgver=$ver
pkgrel=$pkgrel
pkgdesc="$pkgdesc_base"
arch=('x86_64' 'aarch64')
url="$repo_url"
license=('$lic')
$(render_common_depends)
makedepends=('cargo')
# LTO and symbol stripping are configured in Cargo.toml's release profile.
options=('!lto' '!debug')
source=("$src_entry")
sha256sums=('$sum')

_srcname="$src_repo_name-\$pkgver"

$(render_common_build_functions '$_srcname')
EOF
}

render_git_pkgbuild() {
    local ver lic src_entry
    ver=$(compute_git_pkgver)
    lic=$(cargo_license)
    if [[ -n $source_override ]]; then
        src_entry=$source_override
    else
        src_entry="\$_pkgname::git+\$url.git"
    fi

    cat <<EOF
# Maintainer: $maintainer
pkgname=$pkgname_base-git
_pkgname=$pkgname_base
pkgver=$ver
pkgrel=$pkgrel
pkgdesc="$pkgdesc_base (git version)"
arch=('x86_64' 'aarch64')
url="$repo_url"
license=('$lic')
$(render_common_depends)
makedepends=(
    'cargo'
    'git'
)
provides=("\$_pkgname")
conflicts=("\$_pkgname")
# LTO and symbol stripping are configured in Cargo.toml's release profile.
options=('!lto' '!debug')
source=("$src_entry")
sha256sums=('SKIP')

pkgver() {
    cd "\$_pkgname"
    if git describe --long --tags --match 'v[0-9]*' >/dev/null 2>&1; then
        git describe --long --tags --match 'v[0-9]*' --abbrev=7 | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
    else
        printf 'r%s.g%s' "\$(git rev-list --count HEAD)" "\$(git rev-parse --short=7 HEAD)"
    fi
}

$(render_common_build_functions '$_pkgname')
EOF
}

render_aur_gitignore() {
    cat <<'EOF'
# Ignore all makepkg build trees, bare git mirrors, logs, and built packages
*

# Track only AUR packaging files
!.gitignore
!.SRCINFO
!PKGBUILD
EOF
}

render_variant() {
    case $1 in
        release) render_release_pkgbuild ;;
        git) render_git_pkgbuild ;;
        *) die "unknown variant: $1" ;;
    esac
}

variant_dir_name() {
    case $1 in
        release) printf '%s\n' "$pkgname_base" ;;
        git) printf '%s-git\n' "$pkgname_base" ;;
        *) die "unknown variant: $1" ;;
    esac
}

ensure_aur_git_repo() {
    local dir=$1 name=$2
    ((git_init)) || return 0
    require git

    if [[ ! -d "$dir/.git" ]]; then
        info "Initializing standalone AUR Git repository in packaging/arch/$name (branch: master)"
        git init -q -b master "$dir"
        local user_name user_email
        user_name=$(git -C "$repo_root" config user.name || true)
        user_email=$(git -C "$repo_root" config user.email || true)
        [[ -n $user_name ]] && git -C "$dir" config user.name "$user_name"
        [[ -n $user_email ]] && git -C "$dir" config user.email "$user_email"
    fi
}

generate_aur_repo() {
    local variant=$1
    local name dir
    name=$(variant_dir_name "$variant")
    dir="$arch_dir/$name"

    mkdir -p "$dir"
    ensure_aur_git_repo "$dir" "$name"

    render_aur_gitignore >"$dir/.gitignore"
    render_variant "$variant" >"$dir/PKGBUILD"
    bash -n "$dir/PKGBUILD"

    if [[ $variant == "release" ]] && ((run_updpkgsums)); then
        require updpkgsums
        info "Updating SHA-256 checksums for $name via updpkgsums"
        (cd "$dir" && updpkgsums)
    fi

    if command -v makepkg >/dev/null 2>&1; then
        (cd "$dir" && makepkg --printsrcinfo >"$dir/.SRCINFO")
        info "Generated packaging/arch/$name/{PKGBUILD,.SRCINFO,.gitignore}"
    else
        warn "makepkg not found; wrote packaging/arch/$name/PKGBUILD without regenerating .SRCINFO"
    fi

    if [[ $variant == "release" ]] && grep -q "^sha256sums=('SKIP')" "$dir/PKGBUILD"; then
        warn "$name/PKGBUILD currently has sha256sums=('SKIP'); before pushing '$name' to AUR," \
            "tag v$(cargo_version) on GitHub and run: packaging/arch/generate.sh release --updpkgsums"
    fi
}

check_aur_repo() {
    local variant=$1
    local name dir
    name=$(variant_dir_name "$variant")
    dir="$arch_dir/$name"

    require bash
    require makepkg

    [[ -d "$dir" ]] || die "missing AUR directory: packaging/arch/$name (run packaging/arch/generate.sh first)"
    [[ -f "$dir/PKGBUILD" ]] || die "missing packaging/arch/$name/PKGBUILD"
    [[ -s "$dir/.SRCINFO" ]] || die "missing or empty packaging/arch/$name/.SRCINFO"

    info "Checking packaging/arch/$name"
    bash -n "$dir/PKGBUILD"

    local expected_srcinfo
    expected_srcinfo=$(cd "$dir" && makepkg --printsrcinfo)
    if ! diff -u "$dir/.SRCINFO" <(printf '%s\n' "$expected_srcinfo"); then
        die "packaging/arch/$name/.SRCINFO is out of sync with PKGBUILD"
    fi

    if [[ -d "$dir/.git" ]]; then
        require git
        local branch bad_files
        branch=$(git -C "$dir" symbolic-ref --short HEAD 2>/dev/null || true)
        [[ $branch == "master" ]] ||
            die "packaging/arch/$name must be on branch 'master' for AUR (found '${branch:-detached}')"

        bad_files=$(git -C "$dir" ls-files | grep -Ev '^(\.gitignore|\.SRCINFO|PKGBUILD)$' || true)
        if [[ -n $bad_files ]]; then
            die "unexpected tracked files in packaging/arch/$name: $bad_files"
        fi
    else
        warn "packaging/arch/$name/.git is not initialized"
    fi

    if command -v namcap >/dev/null 2>&1; then
        namcap "$dir/PKGBUILD"
    fi
}

if ((check_only)); then
    for t in "${targets[@]}"; do
        check_aur_repo "$t"
    done
    info "All requested AUR repositories passed checks"
    exit 0
fi

if ((stdout_only)); then
    render_variant "${targets[0]}"
    exit 0
fi

if [[ -n $output_file ]]; then
    mkdir -p "$(dirname -- "$output_file")"
    render_variant "${targets[0]}" >"$output_file"
    bash -n "$output_file"
    exit 0
fi

for t in "${targets[@]}"; do
    generate_aur_repo "$t"
done

