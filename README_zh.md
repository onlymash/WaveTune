# WaveTune

[English](README.md) | **简体中文**

面向 Linux 桌面的非官方多品牌无线耳机控制中心（移植自 [EncoDeck](https://github.com/TIANLI0/EncoDeck)），使用 Rust 与 GTK4 / Libadwaita 构建。当前支持 OPPO、OnePlus、realme 及 EarFun 无线耳机，底层已完成多品牌架构解耦，后续将逐步扩展更多品牌支持。

> 本项目与各耳机厂商无任何官方关联。私有蓝牙协议细节与硬件验证记录见 [`docs/OPPO_BLUETOOTH_API.md`](docs/OPPO_BLUETOOTH_API.md) 及 [`docs/EARFUN_BLUETOOTH_API.md`](docs/EARFUN_BLUETOOTH_API.md)。

## 功能特性

- **状态监控与模型预览**：实时显示左右耳与充电盒独立电量、充电状态、佩戴/入耳及盒盖状态、固件版本，支持设备配图与交互式 3D 模型预览。
- **噪声控制**：支持主动降噪（多档降噪深度）、通透模式（含人声增强）与自适应降噪切换。
- **音质与音效**：支持内置调音预设切换、自定义多段均衡器（EQ）、空间音频（固定 / 头部追踪）以及低延迟游戏模式。
- **触控与实用工具**：支持自定义左右耳触控与通话手势、长按降噪循环模式、双设备连接与已配对主机切换、入耳检测、入睡检测、耳塞贴合度检测、查找耳机及提示音音量调节（具体可用功能视耳机型号而定）。
- **桌面集成**：原生 GTK4 / Libadwaita 自适应界面，内置中英双语与多款应用图标样式切换，支持后台常驻与 StatusNotifierItem 系统托盘、登录静默自启，并提供配套 GNOME Shell 右上角快捷面板（Quick Settings）扩展（支持 GNOME 45–51）。

## 安装

### Arch Linux (AUR)

Arch Linux 及其衍生发行版可直接通过 AUR 安装 [`wavetune-git`](https://aur.archlinux.org/packages/wavetune-git)：

```bash
paru -S wavetune-git
# 或使用 yay
yay -S wavetune-git
```

也可手动克隆构建安装：

```bash
git clone https://aur.archlinux.org/wavetune-git.git
cd wavetune-git
makepkg -si
```

使用前请确保系统 `bluetooth` 服务已启用，并先在桌面系统蓝牙设置中完成耳机的配对与连接（WaveTune 负责连接已配对耳机的控制通道，不接管系统蓝牙配对）：

```bash
sudo systemctl enable --now bluetooth
```

## GNOME 快捷面板（Quick Settings）扩展

仓库内置适用于 GNOME 45–51 的配套扩展 `gnome-shell/wavetune@onlymash.github.com`（通过安装包安装时会自动部署到系统扩展目录）。

从源码开发时可手动安装并启用：

```bash
./gnome-shell/install-local.sh
gnome-extensions enable wavetune@onlymash.github.com
```

> Wayland 会话下首次安装新扩展时，若 `gnome-extensions` 尚未识别该 UUID，重新登录一次桌面会话后再执行启用命令即可。

连接受支持的耳机后，右上角快捷设置面板会自动显示耳机磁贴，支持一键开关降噪、查看实时电量、切换降噪子模式或唤起主窗口。应用默认开启「保持后台运行」（关闭窗口隐藏至后台，可通过主菜单「退出」或 `Ctrl+Q` 完全退出），并可在首选项中开启「登录时后台自启」。

## 系统托盘

WaveTune 运行期间会在会话总线上导出 StatusNotifierItem 与 DBusMenu，因此主窗口隐藏到后台后仍可通过托盘访问。托盘使用单色 `io.github.onlymash.WaveTune-symbolic` 图标，提供打开主窗口、查看设备与电量信息、切换降噪模式（需耳机已佩戴且控制可用）以及退出应用等菜单项。

GNOME 默认不提供系统托盘，请安装 [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/) 扩展。KDE Plasma、XFCE、Waybar（Sway/Hyprland/Niri）、COSMIC 等支持 StatusNotifierItem 的桌面环境无需额外软件即可显示。该实现复用现有的 `gio::DBusConnection`，不增加任何运行时或构建依赖。

## 从源码构建

需要系统安装 `bluez`、`gtk4`、`libadwaita`（`>= 1.9`）、`pkgconf` 以及 Rust `stable` 工具链。以 Arch Linux 为例：

```bash
sudo pacman -S --needed bluez gtk4 libadwaita pkgconf rustup
rustup default stable
```

编译与运行：

```bash
cargo build --release   # 发布产物位于 target/release/wavetune
cargo test
cargo run
```

常用命令行参数：

- `--background` / `-b`：静默在后台启动，不弹出主窗口。
- `--probe`：对当前已连接的耳机执行一次只读状态探测并输出终端摘要。

## 打包

打包脚本与配置位于 `packaging/`：

```bash
packaging/package.sh            # 有 makepkg 时构建 Arch 包，否则构建 deb
packaging/package.sh arch       # 构建本地工作区快照 Arch 包
packaging/package.sh git        # 从本地已提交 HEAD 构建 wavetune-git
packaging/package.sh deb        # 构建 Debian/Ubuntu .deb 包（需 dpkg-deb 及 libadwaita >= 1.9）
```

构建产物统一输出至 `target/dist/`。AUR 仓库生成与发布流程详见 [`packaging/arch/README.md`](packaging/arch/README.md)。

## 兼容性说明

内置型号目录涵盖 130 余款 OPPO、OnePlus 和 realme 耳机配置，以及 EarFun 系列（EarFun Air Pro 4、EarFun Free Pro 3、EarFun Air S）。界面会根据耳机上报的产品 ID、能力位与型号配置动态开放对应功能；未识别的新型号将以只读安全模式连接，仅执行握手与产品识别。

## 许可证

GPL-3.0-only
