# EarFun 耳机支持实施计划与开发规范

## 1. 目标与背景

本项目当前已全面完成 OPPO 家族（OPPO、OnePlus、realme）耳机的支持与多品牌架构隔离。
为了拓展多品牌支持，本项目将 **EarFun**（重点为 **EarFun Air Pro 4**）作为首个新加入的独立品牌体系进行完整适配。

底层协议依据已记录在 [EarFun 蓝牙 API 规范](EARFUN_BLUETOOTH_API.md)。EarFun 采用高通 GAIA（Generic Access Profile Interface Architecture）V3 变种协议，基于标准蓝牙 RFCOMM 串口通信（Channel 11）。

---

## 2. 架构设计规范

### 2.1 目录组织与隔离原则
遵循 WaveTune 的品牌分层与隔离原则：
1. **协议层 (`src/protocol/brands/earfun/`)**：
   - 纯 Rust 编解码逻辑，独立于 GTK4 和 BlueZ。
   - `earfun_packet.rs`：GAIA V3 帧结构（`0xFF` SOP + Version + Flags + Length + Vendor ID + Command ID + Payload）的编解码与增量流式解析器（`EarFunFrameDecoder`）。
   - `earfun_commands.rs`：命令常量定义。
   - `earfun_parsers.rs`：电量、固件版本、降噪模式、游戏模式、双设备状态、EQ 数据的解析器与编码器。
2. **兼容层 (`src/compatibility/brands/earfun/`)**：
   - `earfun_catalog.rs`：定义 EarFun 型号列表（以 Product ID `0x4535` 标识 Air Pro 4）。
   - 基于 `features: &'static [&'static str]` 声明各型号支持的能力集合（如 `features::NOISE_CONTROL`, `features::GAME_MODE`, `features::DUAL_DEVICE`, `features::EQUALIZER` 等）。
   - 在 `src/compatibility/registry.rs` 中通过迭代器合并入全局内置型号目录。
3. **传输与会话层 (`src/bluetooth/brands/earfun/`)**：
   - `earfun_session.rs`：负责与 EarFun 耳机建立 RFCOMM Socket 连接（默认 Channel 11）、会话初始化握手、状态读写控制、回读延迟重试（Settle Delays 250ms/500ms）及后台事件监听。
4. **统一蓝牙调度解耦 (`src/bluetooth/`)**：
   - `DeviceBrand` 增加 `EarFun` 枚举项。
   - `discovery.rs` 增加 EarFun 厂商 ID `0x5A70`、GAIA UUID `00001107-D102-11E1-9B23-00025B00A5A5` 及设备名称识别。
   - `device_manager.rs` 根据设备品牌分发至对应的 `spawn_earfun_monitor` 或 `spawn_oppo_monitor`。

### 2.2 隐私与安全规范
- 严禁在日志、代码注释、文档或持久化存储中输出真实耳机的蓝牙 MAC 地址、序列号或敏感设备名。
- 测试用例使用 Mock 虚拟 UnixStream 模拟报文，不得依赖外连真实硬件。

---

## 3. 四阶段实施路线

### 阶段一：基础编解码与数据模型（Protocol & Compatibility）
- [x] 创建 `src/protocol/brands/earfun/` 协议模块：
  - 实现 `EarFunPacket`（包括 `encode` 与流式 `decode`，支持多包粘包与不完整包重组）。
  - 实现各命令 ID 常量定义及 `parse_*` / `encode_*` 函数。
  - 编写协议单元测试，使用真实捕获数据验证边界与状态解析。
- [x] 创建 `src/compatibility/brands/earfun/` 兼容模块：
  - 定义 `EARFUN_DEVICE_PROFILES` 目录，注册全部 Gadgetbridge 支持的 EarFun 型号：
    - **EarFun Air Pro 4**（PID `0x004535`）：10 段硬件 EQ、三路电量（左/右/盒）、10 级降噪树、佩戴检测、双设备、游戏模式、查找耳机等。
    - **EarFun Free Pro 3**（PID `0x004534`）：10 段硬件 EQ、三路电量（左/右/盒）、10 级降噪树、双设备、游戏模式、查找耳机（无佩戴检测）。
    - **EarFun Air S**（PID `0x004c45`）：6 段硬件 EQ（附加 4 个 Dummy bands 补齐 10 段）、两路电量（仅左/右耳，无盒电量）、3 级扁平降噪模式（关/降噪/通透）、双设备、游戏模式（无佩戴检测）。
  - 在 `src/compatibility/registry.rs` 中通过迭代器合并入全局内置型号目录。

### 阶段二：蓝牙发现路由与会话生命周期（Bluetooth & Session）
- [x] 扩展 `src/bluetooth/` 核心调度：
  - 在 `DeviceBrand` 中增加 `EarFun`。
  - 在 `discovery.rs` 中增加 EarFun 厂商识别规则（Vendor ID `0x5A70`、名称前缀、GAIA UUID）。
  - 重构 `spawn_monitor`，根据目标设备的 `DeviceBrand` 动态分发至对应品牌的监视器。
- [x] 实现 `src/bluetooth/brands/earfun/earfun_session.rs`：
  - 建立 RFCOMM Channel 11 连接。
  - 执行初始化查询序列：固件版本 (`0x0309`)、电量（针对 Air S 仅查左/右耳 `0x0306`, `0x0307`；Pro 4/Free Pro 3 额外查盒电量 `0x0317`）、降噪状态 (`0x0315`, `0x033B`, `0x033D`)、游戏模式 (`0x0313`)、入耳检测 (`0x034A`，仅在型号支持时查询)、双设备 (`0x0326`)。
  - 组装初始 `EarFunDeviceSnapshot` 同步至前端 UI。

### 阶段三：状态同步与功能写入控制（Controls & Readback）
- [x] 在 `earfun_session.rs` 中实现写操作逻辑：
  - `set_noise_mode`：下发主模式 (`0x0314`) 与子模式 (`0x033A` / `0x033C`)（Air S 则直接下发 3 级主模式），等待响应并通过回读确认。
  - `set_game_mode`：下发 `0x0312`。
  - `set_in_ear_detection`：下发 `0x0349`（佩戴检测支持型号）。
  - `set_dual_device`：下发 `0x0327`。
  - `set_equalizer`：按频点设置硬件 EQ (`0x0E01`)，对 6 段型号自动填充后 4 个虚拟频段。
- [x] 实现回读 Settle 延迟重试（250ms/500ms），保障控制稳定性。
- [x] 开启长连接轮询/监听任务，向 UI 发送 `UiUpdate` 实时更新电量与状态。

### 阶段四：真机联调与自动化测试（Verification & Tests）
- [x] 编写 Mock 单元测试覆盖所有协议分支与会话状态转移，确保 CI/编译打包环境 100% 通过（100+ 个单元测试全部通过）。
- [x] 命令行只读探针测试（`cargo run -- --probe`）：在已配对的 EarFun Air Pro 4 实机上成功读取三路电量、固件版本、降噪模式、游戏模式、双设备状态与佩戴检测。
- [x] 完善国际化文本与界面配置加载。

