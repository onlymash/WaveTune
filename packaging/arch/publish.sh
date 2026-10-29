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

# Automatically update and publish WaveTune AUR packages (`wavetune` / `wavetune-git`).
#
# Usage:
#   packaging/arch/publish.sh [git|release|all] [options]
#
# Targets:
#   git, wavetune-git      Update and publish wavetune-git to AUR
#   release, wavetune      Update (with pinned SHA-256) and publish wavetune to AUR
#   all                    Update and publish both wavetune and wavetune-git
#                          (With no target: publishes `git`, plus `release` if tag v<version>
#                          already exists on GitHub `origin`)
#
# Options:
#   --pkgrel <n>           Set pkgrel (default: 1)
#   --tag                  For `release`: create and push tag `v<version>` to GitHub `origin` if missing
#   -m, --message <msg>    Override the AUR git commit message
#   --no-origin            Skip syncing to backup remote `origin` (if configured in the AUR repo)
#   -n, --dry-run          Generate files and show planned commit/push without network writes
#   -h, --help             Show this help message
set -euo pipefail
umask 022

arch_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$arch_dir/../.." && pwd)

aur_host="aur@aur.archlinux.org"
pkgrel="1"
create_tag=0
custom_message=""
push_origin=1
dry_run=0
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
        --pkgrel)
            (($# >= 2)) || die "missing argument for $1"
            pkgrel=$2
            shift
            ;;
        --tag)
            create_tag=1
            ;;
        -m | --message)
            (($# >= 2)) || die "missing argument for $1"
            custom_message=$2
            shift
            ;;
        --no-origin)
            push_origin=0
            ;;
        -n | --dry-run)
            dry_run=1
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

require git
require ssh
require makepkg

cargo_version() {
    local ver
    ver=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)
    [[ -n $ver ]] || die "could not read package version from $repo_root/Cargo.toml"
    printf '%s\n' "$ver"
}

remote_tag_exists() {
    local tag=$1
    git -C "$repo_root" ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1
}

verify_aur_ssh() {
    info "Verifying SSH authentication with $aur_host"
    local ssh_out
    ssh_out=$(ssh -o BatchMode=yes -o ConnectTimeout=15 -T "$aur_host" 2>&1 || true)
    if [[ $ssh_out == *"Welcome to AUR"* ]]; then
        local first_line=${ssh_out%%$'\n'*}
        info "$first_line"
    else
        printf '%s\n' "$ssh_out" >&2
        die "AUR SSH authentication failed. Test manually with: ssh -T $aur_host"
    fi
}

variant_dir_name() {
    case $1 in
        release) printf 'wavetune\n' ;;
        git) printf 'wavetune-git\n' ;;
        *) die "unknown variant: $1" ;;
    esac
}

ensure_git_identity() {
    local dir=$1
    local user_name user_email
    user_name=$(git -C "$dir" config user.name || git -C "$repo_root" config user.name || true)
    user_email=$(git -C "$dir" config user.email || git -C "$repo_root" config user.email || true)
    [[ -n $user_name && -n $user_email ]] ||
        die "git user.name / user.email not configured; set them with git config"
    git -C "$dir" config user.name "$user_name"
    git -C "$dir" config user.email "$user_email"
}

sync_aur_repo() {
    local name=$1 dir=$2 aur_url=$3
    mkdir -p "$dir"
    if [[ ! -d "$dir/.git" ]]; then
        git init -q -b master "$dir"
    fi
    ensure_git_identity "$dir"

    if git -C "$dir" remote get-url aur >/dev/null 2>&1; then
        git -C "$dir" remote set-url aur "$aur_url"
    else
        git -C "$dir" remote add aur "$aur_url"
    fi

    if ((dry_run)); then
        return 0
    fi

    if git -C "$dir" ls-remote --exit-code aur refs/heads/master >/dev/null 2>&1; then
        info "Fetching existing AUR history for $name"
        git -C "$dir" fetch -q aur master
        if ! git -C "$dir" rev-parse --verify HEAD >/dev/null 2>&1; then
            git -C "$dir" checkout -q -B master aur/master
        elif git -C "$dir" merge-base --is-ancestor HEAD aur/master; then
            git -C "$dir" merge --ff-only -q aur/master
        elif ! git -C "$dir" merge-base --is-ancestor aur/master HEAD; then
            # Local repo was initialized independently on another machine; align history to aur/master
            # while keeping working tree ready to be overwritten by generate.sh.
            warn "Aligning local $name history onto aur/master"
            git -C "$dir" reset --soft aur/master
        fi
    else
        info "$name does not exist on AUR yet (first push will create the package)"
    fi
}

ensure_release_tag() {
    local ver tag
    ver=$(cargo_version)
    tag="v$ver"

    if ((dry_run)); then
        if ((create_tag)); then
            info "[dry-run] Would ensure tag $tag exists and push it to GitHub origin"
        fi
        return 0
    fi

    if remote_tag_exists "$tag"; then
        return 0
    fi

    if ((!create_tag)); then
        die "Release tag '$tag' is not on GitHub origin yet. Push the tag first or re-run with: packaging/arch/publish.sh release --tag"
    fi

    if [[ -n $(git -C "$repo_root" status --porcelain --untracked-files=no) ]]; then
        die "WaveTune working tree has uncommitted changes; commit them before tagging $tag"
    fi

    if ! git -C "$repo_root" rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        info "Creating release tag $tag on HEAD ($(git -C "$repo_root" rev-parse --short HEAD))"
        git -C "$repo_root" tag "$tag"
    fi

    info "Pushing $tag to GitHub origin"
    git -C "$repo_root" push origin "$tag"
}

publish_variant() {
    local variant=$1
    local name dir aur_url
    name=$(variant_dir_name "$variant")
    dir="$arch_dir/$name"
    aur_url="ssh://$aur_host/$name.git"

    if [[ $variant == "release" ]]; then
        ensure_release_tag
    fi

    sync_aur_repo "$name" "$dir" "$aur_url"

    local gen_args=("$variant" --pkgrel "$pkgrel")
    if [[ $variant == "release" ]] && ((!dry_run)); then
        gen_args+=(--updpkgsums)
    fi

    bash "$arch_dir/generate.sh" "${gen_args[@]}"
    bash "$arch_dir/generate.sh" "$variant" --check

    if [[ $variant == "release" ]] && ((!dry_run)); then
        if grep -q "^sha256sums=('SKIP')" "$dir/PKGBUILD"; then
            die "$name/PKGBUILD still has sha256sums=('SKIP'); refusing to publish incomplete checksum to AUR"
        fi
    fi

    git -C "$dir" add .gitignore PKGBUILD .SRCINFO

    local cur_ver cur_rel commit_msg has_head=0
    cur_ver=$(sed -n 's/^pkgver=//p' "$dir/PKGBUILD" | head -n1)
    cur_rel=$(sed -n 's/^pkgrel=//p' "$dir/PKGBUILD" | head -n1)
    if git -C "$dir" rev-parse --verify HEAD >/dev/null 2>&1; then
        has_head=1
    fi

    if [[ -n $custom_message ]]; then
        commit_msg=$custom_message
    elif ((!has_head)); then
        commit_msg="Initial import: $name $cur_ver-$cur_rel"
    else
        commit_msg="upgpkg: $name $cur_ver-$cur_rel"
    fi

    if ((dry_run)); then
        if ! git -C "$dir" diff --cached --quiet; then
            info "[dry-run] Changes staged for $name ($commit_msg):"
            git -C "$dir" status --short
            git -C "$dir" reset -q
        else
            info "[dry-run] No metadata changes in $name ($cur_ver-$cur_rel)"
        fi
        info "[dry-run] Would push master -> $aur_url"
        return 0
    fi

    if ! git -C "$dir" diff --cached --quiet; then
        info "Committing $name: $commit_msg"
        git -C "$dir" commit -m "$commit_msg"
    fi

    local local_head aur_head
    local_head=$(git -C "$dir" rev-parse HEAD)
    aur_head=$(git -C "$dir" rev-parse refs/remotes/aur/master 2>/dev/null || true)

    if [[ $local_head != "$aur_head" ]]; then
        info "Pushing $name ($cur_ver-$cur_rel) to AUR ($aur_url)"
        git -C "$dir" push -u aur master
        info "Published: https://aur.archlinux.org/packages/$name"
    else
        info "$name ($cur_ver-$cur_rel) is already up to date on AUR"
    fi

    if ((push_origin)) && git -C "$dir" remote get-url origin >/dev/null 2>&1; then
        local origin_url
        origin_url=$(git -C "$dir" remote get-url origin)
        if [[ $origin_url != "$aur_url" ]]; then
            info "Syncing $name to backup remote 'origin' ($origin_url)"
            git -C "$dir" push origin master
        fi
    fi
}

if ((${#targets[@]} == 0)); then
    ver=$(cargo_version)
    if ((create_tag)); then
        targets=(release git)
    elif ((!dry_run)) && remote_tag_exists "v$ver"; then
        targets=(release git)
    else
        targets=(git)
        info "Tag v$ver not published on GitHub yet; defaulting to 'wavetune-git' (use 'release --tag' to publish release)"
    fi
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

if ((!dry_run)); then
    verify_aur_ssh
fi

for t in "${targets[@]}"; do
    publish_variant "$t"
done

