# WaveTune Arch Linux / AUR 打包与自动发布说明

本目录下的脚本属于 WaveTune 主仓库的一部分，用于自动生成、更新并发布两套独立的 AUR 软件包：

- [`generate.sh`](generate.sh)：根据 `Cargo.toml` 与 Git 元数据生成 `PKGBUILD`、`.SRCINFO` 与 `.gitignore`。
- [`publish.sh`](publish.sh)：一键完成远端同步、版本/校验和更新、规范检查、自动提交并推送到 AUR（若配置了 GitHub 备份远端 `origin` 也会自动同步）。

生成的目标仓库目录已在主仓库 `.gitignore` 中忽略：

- `packaging/arch/wavetune/`：正式源码发布版（基于 GitHub Release Tag `v<version>` 构建）
- `packaging/arch/wavetune-git/`：开发版（跟踪 GitHub `master` 分支最新提交构建）

## 一键自动更新与发布到 AUR（推荐）

确保本机 SSH 公钥已添加到 [aur.archlinux.org](https://aur.archlinux.org/) 账号后，直接在主仓库根目录执行：

```bash
# 默认模式：若当前版本已在 GitHub 打过 v<version> Tag 则同时更新 wavetune 与 wavetune-git；否则仅更新 wavetune-git
packaging/arch/publish.sh

# 仅更新并发布 wavetune-git（开发版）
packaging/arch/publish.sh git

# 发布正式版 wavetune（若尚未在 GitHub 推送 v<version> Tag，加上 --tag 会自动打 Tag 并推送）
packaging/arch/publish.sh release --tag

# 已手动推送过 GitHub Tag 时，直接拉取计算真实 SHA-256 并发布正式版
packaging/arch/publish.sh release

# 同时更新并发布正式版与开发版
packaging/arch/publish.sh all

# 预演模式（仅生成文件并展示变更计划，不执行联网拉取、打 Tag、提交或推送）
packaging/arch/publish.sh --dry-run
```

> **多设备协同支持**：在新电脑克隆 WaveTune 后无需手动初始化子仓库，直接运行 `packaging/arch/publish.sh` 即会自动拉取线上 `aur/master` 历史并对齐提交，不会产生历史冲突。

## 仅在本地生成或校验文件（不推送）

```bash
# 同时生成 wavetune 与 wavetune-git 目录及元数据
packaging/arch/generate.sh

# 仅生成正式版 / 仅生成开发版
packaging/arch/generate.sh release
packaging/arch/generate.sh git

# 正式版打好 GitHub Tag 后更新真实 SHA-256 校验和
packaging/arch/generate.sh release --updpkgsums

# 校验 PKGBUILD 语法、.SRCINFO 一致性及 AUR 分支/白名单规范
packaging/arch/generate.sh --check
```
