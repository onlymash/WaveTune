# WaveTune Arch Linux / AUR 打包生成与维护说明

本目录下的 [`generate.sh`](generate.sh) 属于 WaveTune 主仓库的一部分，用于自动生成两套独立的 AUR 仓库内容：

- `packaging/arch/wavetune/`：正式源码发布版（非 `-git`，基于 GitHub Release Tag `v<version>` 构建）
- `packaging/arch/wavetune-git/`：开发版（跟踪 GitHub `master` 分支最新提交构建）

> 上述两个生成目录已在主仓库 `.gitignore` 中忽略，各自作为独立的 AUR Git 仓库（`master` 分支，仅跟踪 `PKGBUILD`、`.SRCINFO`、`.gitignore`）直接推送至 `aur.archlinux.org`。

## 生成 PKGBUILD 与 AUR 仓库

在项目根目录执行：

```bash
# 同时生成 wavetune（正式版）和 wavetune-git（开发版）两个 AUR 仓库目录
packaging/arch/generate.sh

# 仅生成非 git 正式版（版本号自动读取自 Cargo.toml）
packaging/arch/generate.sh release

# 正式版打好 GitHub v<version> Tag 后，自动拉取源码包写入真实 SHA-256 并更新 .SRCINFO
packaging/arch/generate.sh release --updpkgsums

# 仅生成 -git 开发版（初始 pkgver 自动按当前本地 HEAD 计算）
packaging/arch/generate.sh git

# 校验已生成的 AUR 仓库（PKGBUILD 语法、.SRCINFO 一致性、master 分支与白名单文件检查）
packaging/arch/generate.sh --check
```

若只需单独输出某一份 `PKGBUILD` 文件（例如供 [`packaging/package.sh`](../package.sh) 在 `target/dist/` 中构建调用）：

```bash
packaging/arch/generate.sh release --output target/dist/arch/PKGBUILD
packaging/arch/generate.sh git --stdout
```

## 首次发布到 AUR

确保 GitHub 仓库 `https://github.com/onlymash/WaveTune` 已设为 **Public**，且已在 [aur.archlinux.org](https://aur.archlinux.org/) 配置好 SSH 公钥。

### 发布 `wavetune-git`（开发版）

```bash
packaging/arch/generate.sh git
packaging/arch/generate.sh git --check

cd packaging/arch/wavetune-git
git remote add origin ssh://aur@aur.archlinux.org/wavetune-git.git
git add .gitignore PKGBUILD .SRCINFO
git commit -m "Initial import: wavetune-git $(sed -n 's/^pkgver=//p' PKGBUILD)-1"
git push -u origin master
```

### 发布 `wavetune`（正式版）

正式版需要先在主仓库完成版本发布并推送 GitHub Tag（例如 `v0.1.0`），以便锁定真实的源码包校验和：

```bash
# 1. 在主仓库推送版本 Tag 后，生成正式版并自动计算真实 sha256sums
packaging/arch/generate.sh release --updpkgsums
packaging/arch/generate.sh release --check

# 2. 提交并推送至 AUR 的 wavetune 仓库
cd packaging/arch/wavetune
git remote add origin ssh://aur@aur.archlinux.org/wavetune.git
git add .gitignore PKGBUILD .SRCINFO
git commit -m "Initial import: wavetune $(sed -n 's/^pkgver=//p' PKGBUILD)-1"
git push -u origin master
```

## 后续版本更新

`generate.sh` 在已存在 `.git` 目录时不会覆盖已有 Git 历史与远端配置，重新执行即可原地更新 `PKGBUILD` 和 `.SRCINFO`：

```bash
# 正式版发版更新（先更新 Cargo.toml 版本并推送新 Tag）
packaging/arch/generate.sh release --updpkgsums
cd packaging/arch/wavetune
git add PKGBUILD .SRCINFO
git commit -m "upgpkg: wavetune $(sed -n 's/^pkgver=//p' PKGBUILD)-1"
git push

# -git 版依赖或构建脚本变更时更新
packaging/arch/generate.sh git
cd packaging/arch/wavetune-git
git add PKGBUILD .SRCINFO
git commit -m "upgpkg: wavetune-git $(sed -n 's/^pkgver=//p' PKGBUILD)-1"
git push
```

