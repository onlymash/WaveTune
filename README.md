# WaveTune

一个面向 Linux 桌面的非官方多品牌无线耳机控制中心（移植自 [EncoDeck](https://github.com/TIANLI0/EncoDeck)），
使用 Rust 与 GTK4/Libadwaita 构建。当前首要适配 OPPO、OnePlus 和 realme（欧加体系）耳机，底层架构已完成
品牌解耦隔离，未来将持续扩展其它品牌耳机。当前开发环境目标为 Arch Linux + GNOME。

> 该项目与各耳机厂商无官方关联。OPPO 私有蓝牙协议与已知证据范围见
> [协议记录](docs/OPPO_BLUETOOTH_API.md)，实施阶段见[开发计划](docs/IMPLEMENTATION_PLAN.md)。

## 当前进度

- GTK4 原生窗口和总览、音效、设备信息页面。
- 启动后自动发现由 GNOME/BlueZ 连接的 OnePlus/OPPO/realme 耳机；按 BlueZ 耳机类别与可验证的
  Bluetooth 厂商数据或 OPPO 控制服务 UUID 筛选，名称仅用于列表显示。每 15 秒刷新，也可
  手动刷新；所有识别出的 OnePlus/OPPO/realme 耳机都可尝试连接控制通道。连接时通过 SDP 查找
  `ELNK` RFCOMM 服务和通道；未找到服务时显示明确错误。
- 已导入 TIANLI0/EncoDeck 的 OPPO/OnePlus 型号目录及其 realme 补充目录，共 131 个型号。
  目录驱动现有降噪和双设备管理选项；具体操作仍受耳机能力位、运行时支持列表/功能开关和
  写入确认约束。目录不是硬件兼容性测试报告；X3 仅验证过 RFCOMM 连接、握手和产品识别，
  尚未验证设置写入。X4 已验证只读握手、型号、固件、佩戴/入耳、盒盖、电量和降噪状态查询。
- OPPO 应用包编解码、RFCOMM 帧编码、增量帧解码及协议单元测试。
- 已完成多品牌架构解耦重构（参考 EncoDeck 架构）：将协议层（`src/protocol/`）、兼容配置层（`src/compatibility/`）
  与会话层（`src/bluetooth/`）区隔为通用状态抽象与品牌专有实现；OPPO/OnePlus/realme 欧加私有协议与配置已收敛至
  `brands/oppo/`，类名、结构体名与方法全面规范化（如 `OppoFrameDecoder`、`OppoAppPacket`、`OppoCommands`、
  `OppoDeviceSnapshot`、`OPPO_DEVICE_PROFILES` 等），并通过统一注册中心 `builtin_profiles()` 聚合，为未来扩展华为、
  小米、vivo、漫步者等多品牌耳机支持奠定基础。
- 首页连接后显示真实电量和当前降噪模式；依型号目录生成模式选项，直接使用兼容配置中声明
  的模式集合，不再要求先读取耳机支持模式 API 才开放功能。设置受 capability bit 8、兼容
  配置树、入耳状态门控，并要求 ACK 和回读匹配。EQ 写入仍禁用。

## Arch Linux 开发依赖

当前开发环境已检查为：

- Arch Linux `gtk4` 包：`1:4.24.1-1`（`pkg-config` 报告 GTK `4.24.1`）
- Arch Linux `libadwaita` 包：`1.10.0`（项目需要系统 libadwaita `1.9` 或更新版本）
- Rust stable：`rustc 1.99.1`、`cargo 1.99.1`
- Rust GTK 绑定：`gtk4 0.11.5`

可使用以下命令检查本机环境与软件包版本：

```bash
pacman -Qi gtk4 libadwaita
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
rustup show active-toolchain
rustc --version
cargo --version
```

安装 GTK4、libadwaita 开发库、pkg-config 和 Rustup（如果尚未安装）：

```bash
sudo pacman -S --needed bluez gtk4 libadwaita pkgconf rustup
rustup toolchain install stable
rustup default stable
```

启用 BlueZ 服务：

```bash
sudo systemctl enable --now bluetooth
systemctl is-active bluetooth
```

GTK Rust 绑定使用 crates.io 上的最新稳定版 `0.11.5`；系统 GTK4 由 Arch 的 `gtk4` 包提供。
更新 Arch 软件包和 Rust stable 工具链：

```bash
sudo pacman -Syu
rustup update stable
```

## 构建与运行

```bash
cargo build
cargo test
cargo run
```

`cargo run` 会编译并启动 GTK 应用；在 GNOME Wayland 会话中正常从终端启动即可。要构建
优化后的发布版本，可运行 `cargo build --release`，生成文件位于
`target/release/wavetune`。如果 GTK 或 libadwaita 系统依赖缺失，确认
`pkg-config --modversion gtk4` 和 `pkg-config --modversion libadwaita-1` 能正常输出版本，再重新构建。

## 打包

打包配置位于 `packaging/`：

- `packaging/arch/wavetune/PKGBUILD`：正式版，从 GitHub `v<version>` tag 的源码包构建。
- `packaging/arch/wavetune-git/PKGBUILD`：跟踪最新提交的 `-git` 版本。
- `packaging/debian/`：`.deb` 的 `control` 模板和版权文件。
- `packaging/install.sh`：两种格式共用的安装文件布局。

使用打包脚本从当前工作区快速打包（包含未提交修改和未被 `.gitignore` 忽略的新文件）：

```bash
packaging/package.sh            # 有 makepkg 时构建 Arch 包，否则构建 deb
packaging/package.sh arch       # 仅 Arch 包
packaging/package.sh git        # wavetune-git，从本地仓库已提交的 HEAD 构建
packaging/package.sh deb        # 仅 deb（需要 dpkg-deb）
packaging/package.sh all --no-check
```

产物输出到 `target/dist/`，中间文件位于 `target/dist/<格式>/`，不会污染项目根目录。
`.deb` 链接的是构建主机的 glibc/GTK/libadwaita，发布用的 deb 应在目标 Debian/Ubuntu
发行版上构建（需要 libadwaita >= 1.9）。发布新版本时，同步更新 `Cargo.toml` 与
`packaging/arch/wavetune/PKGBUILD` 中的版本号，打 tag 后在 PKGBUILD 目录运行 `updpkgsums`。

## OnePlus/OPPO/realme 控制通道连接

确保耳机先在 GNOME 蓝牙设置中配对并连接。应用启动后会自动连接第一台发现的、提供控制服务的
耳机（若暂未发现，会在后续定期刷新中继续尝试，直到连接或用户手动选择）；也可在设备列表中手动
选择并切换。未连接控制通道时，四个导航页面均显示空白状态并提示连接耳机。列表会定期及按需刷新。应用不会主动
配对或连接尚未由 GNOME/BlueZ 连接的设备。候选列表按 BlueZ 耳机类型以及 OPPO/OnePlus/realme
Bluetooth Company ID（`0x079A` / `0x072F` / `0x08A4`）或已验证的 OPPO 控制服务 UUID 筛选，而不是按
设备名称筛选；名称只作显示。BlueZ `Modalias` 描述的可能是芯片厂商，不作为耳机品牌依据。
所有识别出的 OnePlus/OPPO/realme 耳机都可尝试连接；控制通道按 SDP 中的 `ELNK` 服务记录解析，
不会猜测 RFCOMM 通道。`--probe` 是一次性只读探测，多台设备时无法交互选择：

```bash
cargo run -- --probe
```

当前静态目录包含 131 个型号：EncoDeck 主目录 85 项、补充目录 46 项；其中 43 个型号含降噪模式树，
51 个型号声明双设备管理。数据来源、修订版本与导入范围见[协议记录](docs/OPPO_BLUETOOTH_API.md)。
目录描述型号配置，不代表该型号已通过硬件验证。已知型号可依据目录开放现有功能，具体按钮仍须通过
耳机能力位图、运行时模式支持清单或双设备开关状态。X4 的 `0x010C` 子类型 3 实测返回空 bitmap，
故回退到 X4 目录；X3 的 RFCOMM、握手和产品识别已验证，但其设置控制尚未真机验证。未知产品 ID
只进行握手和产品识别。降噪操作仍需 capability bit 8、至少一耳入耳、成功 ACK 和匹配回读；
EQ、手势等目录中的其他功能尚未实现。型号兼容配置位于 `src/compatibility/catalog.rs` 和
`src/compatibility/mod.rs`；不会从 X4 推断其他型号的能力。

首页降噪模式下方的“设备管理”以列表显示耳机上报的已配对主机，每项依次显示设备 emoji、名称和连接/断开按钮，
并标记当前电脑。该列表通过 Enco X4 的 `0x0112` 查询和设备列表事件更新；当前会话仅在内存中短暂保留目标蓝牙地址
以发送用户点击的连接/断开命令，不显示或记录地址，也不导出或持久化。仅当耳机报告双设备连接已启用时开放操作；
当前电脑不可从此列表断开。写命令需收到耳机 ACK，并通过 `0x0112` 列表回读确认状态。
已在 OPPO Enco X4 上验证对一台非当前主机断开后立即重连，两个操作均收到 ACK 且列表回读状态一致
（2026-10-22；本次操作验证未记录耳机固件版本）。

## 许可证

GPL-3.0-only。
