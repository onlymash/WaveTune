# WaveTune 蓝牙协议层与多品牌驱动架构重构方案

本文档为 WaveTune 蓝牙通信层（`src/bluetooth`）的系统性重构指南。方案基于 [`docs/ARCHITECTURE_GUIDELINES.md`](ARCHITECTURE_GUIDELINES.md) 的垂直切片（Feature-First）、单一可信状态源（SSOT）与轻量 MVI 原则，结合“驱动子系统与设备驱动抽象（Subsystem & Device Driver Pattern）”模型制定，旨在彻底解决当前 `DeviceSnapshot` 字段爆炸、巨石会话单文件以及跨品牌代码冗余等痛点，为后续接入华为（FreeBuds）、小米、漫步者等多品牌奠定稳固的基础。

---

## 一、 背景与现存问题剖析

### 1. `DeviceSnapshot` 上帝对象（God Object）与状态脱节
- **现状**：`OppoDeviceSnapshot` 包含近 50 个散落字段，`EarFunDeviceSnapshot` 包含 30 多个散落字段。将设备名、产品 ID、固件版本、电池、佩戴状态、降噪模式树、EQ 预设、空间音频、低音增强、手势按键、双设备配对列表、探针错误日志强行平铺在单一结构体中。
- **痛点**：
  - 严重违背业务垂直切片规范；
  - 与应用层状态模型 [`src/app/state.rs`](../src/app/state.rs)（`HeadsetSessionState`，已按 `Noise`、`Sound`、`Controls`、`DualDevice` 分片）结构脱节且字段镜像冗余；
  - 每次新增微小特性均引发“散弹式修改（Shotgun Surgery）”，改动波及数十处构造与解包代码。

### 2. 会话初始化的“事件风暴”与 Worker 内部影子状态（Shadow State）
- **事件风暴**：会话连接建立后，Worker 连续向主线程发送 20~30 个独立的 `UiUpdate` 细碎消息，导致主线程事件循环与各个 Feature Controller 被高频唤醒；
- **违反单一可信状态源（SSOT）**：Worker 线程内部为了后续命令执行的前置校验，单独 `clone` 并维护了 10 多个局部变量（`current_wear`、`current_custom_equalizers`、`current_key_functions` 等），造成底层 Worker 与上层 `HeadsetSessionState` 同时持有双份状态，存在状态漂移隐患。

### 3. `*_session.rs` 巨石单文件与职责混杂
- `oppo_session.rs`（2381 行）与 `earfun_session.rs`（1562 行）将 RFCOMM 传输管理、多步握手状态机、800+ 行的大命令循环 `match commands.try_recv()`、各特性的写入管道（ACK+回读）、主动广播解析及命令行 Probe 打印全部堆积在单文件中，代码维护与协同成本极高。

### 4. 缺乏多品牌驱动抽象机制
- 品牌之间缺乏统一的 Driver Trait 约束，`bluetooth/mod.rs` 与 `probe.rs` 中通过硬编码 `match device.brand` 分发，各品牌重复实现了通用的 I/O 监听、超时重试与通道派发流程。

---

## 二、 目标架构蓝图

重构采用 **“内核驱动子系统模型（Subsystem & Driver Architecture）”**，划分为三层：

```
┌────────────────────────────────────────────────────────────────────────┐
│               WaveTune 通用协议引擎 (Generic Session Runner)            │
│                                                                        │
│  - 统一管理 Linux RFCOMM 套接字生命周期与 SDP 解析                     │
│  - 统一管理主线程通信桥接（WorkerCommand 接收、UiUpdate 发送）         │
│  - 会话初始化原子同步：UiUpdate::InitialSession(Box<DeviceSnapshot>)   │
│  - 统一的命令分发与通用异常捕获                                        │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │ 调用标准接口
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   统一耳机驱动接口 (HeadsetDriver Trait)                │
│                                                                        │
│   pub trait HeadsetDriver: Send + 'static {                            │
│       fn initialize(&mut self, stream) -> Result<DeviceSnapshot, ...>; │
│       fn handle_command(&mut self, stream, WorkerCommand) -> ...;      │
│       fn handle_unsolicited(&mut self, stream, Frame) -> ...;          │
│   }                                                                    │
└──────┬───────────────────────────┼──────────────────────────────┬──────┘
       │                           │                              │
       ▼ [垂直切片组织]             ▼ [垂直切片组织]               ▼ [垂直切片组织]
┌──────────────┐            ┌──────────────┐              ┌──────────────┐
│  OPPO 驱动   │            │ EarFun 驱动  │              │ 未来：华为驱动 │
│ (OnePlus等)  │            │              │              │ (FreeBuds)   │
│ ├── init     │            │ ├── init     │              │ ├── init     │
│ ├── noise    │            │ ├── noise    │              │ ├── noise    │
│ └── sound... │            │ └── sound... │              │ └── sound... │
└──────────────┘            └──────────────┘              └──────────────┘
       ▲                           ▲                              ▲
       └───────────────────────────┴──────────────────────────────┘
                    输出统一模型：领域切片化 DeviceSnapshot
```

---

## 三、 分阶段实施路线图

为了保证系统稳定性并确保现有 110+ 个单元测试始终处于全绿状态，重构分为五个阶段演进：

### Phase 1: 数据模型解耦与统一切片快照 (Data Layer Refactoring)

#### 目标
将平铺的 `DeviceSnapshot` 解耦为结构化切片模型，统一向 `HeadsetSessionState` 映射，消除初始化事件风暴。

#### 详细任务
1. **新建切片快照模型** [`src/bluetooth/snapshot.rs`](../src/bluetooth/)：
   ```rust
   pub struct DeviceSnapshot {
       pub identity: DeviceIdentitySnapshot,
       pub battery: BatteryState,
       pub wear: WearSnapshot,
       pub noise: NoiseSnapshot,
       pub sound: SoundSnapshot,
       pub controls: ControlsSnapshot,
       pub dual_device: DualDeviceSnapshot,
       pub probe_diagnostics: ProbeDiagnostics,
   }
   ```
2. **状态层原子同步支持**：
   - 在 [`src/app/events.rs`](../src/app/events.rs) 中新增 `UiUpdate::InitialSession(Box<DeviceSnapshot>)`；
   - 在 [`src/app/state.rs`](../src/app/state.rs) 中为 `HeadsetSessionState` 实现 `apply_snapshot(&mut self, snapshot: &DeviceSnapshot)`，单步完成全量视图状态填充；
3. **消除会话初期的“事件风暴”**：
   - 将 `monitor_device` 中连续发送 20 余个细碎 `UiUpdate` 的逻辑，替换为单次 `UiUpdate::InitialSession` 派发；
4. **统一 `--probe` 命令行输出**：
   - 重构 [`src/bluetooth/probe.rs`](../src/bluetooth/probe.rs)，使其直接接收通用的 `DeviceSnapshot` 进行标准化格式化打印，删除重复的 `print_earfun_snapshot`；
5. **保证兼容性**：
   - 在旧结构上保留类型别名或转换器，确保现有单元测试无缝通过。

#### 验收标准
- 现有单元测试及 probe 功能 100% 编译通过；
- 会话建立时，主线程更新事件由 25+ 次收敛为 1 次原子更新。

---

### Phase 2: 驱动核心接口与通用会话执行器 (Driver Abstraction & Runner)

#### 目标
提炼跨品牌公共的驱动 Trait 与通用的会话执行循环（`SessionRunner`），将通用的线程生命周期和命令调度与品牌专用协议隔离。

#### 详细任务
1. **定义抽象驱动接口** [`src/bluetooth/driver.rs`](../src/bluetooth/)：
   ```rust
   pub trait HeadsetDriver: Send + 'static {
       /// 执行初次握手、型号识别与能力探测，构建切片快照
       fn initialize<T: Read + Write>(
           &mut self,
           stream: &mut T,
           updates: &mpsc::Sender<UiUpdate>,
       ) -> Result<DeviceSnapshot, String>;

       /// 执行用户在 UI 触发的写命令（包含单指令安全写管道与回读）
       fn handle_command<T: Read + Write>(
           &mut self,
           stream: &mut T,
           command: WorkerCommand,
           updates: &mpsc::Sender<UiUpdate>,
       ) -> Result<(), String>;

       /// 解析设备主动上报的未请求数据包（如佩戴、电量变化等广播）
       fn handle_unsolicited<T: Write>(
           &mut self,
           stream: &mut T,
           frame: &[u8],
           updates: &mpsc::Sender<UiUpdate>,
       ) -> Result<Option<UiUpdate>, String>;
   }
   ```
2. **构建通用会话执行器** [`src/bluetooth/worker.rs`](../src/bluetooth/)：
   - 封装 `run_session_worker(device, driver, updates, commands)`：
     - 自动管理 RFCOMM 打开、SDP 匹配与断开提示；
     - 执行 `driver.initialize()` 并发出 `InitialSession`；
     - 统一处理 `commands.try_recv()` 与底层 socket poll，调度 `handle_command` 与 `handle_unsolicited`；
     - 捕获异常并标准化向 UI 抛出 `UiUpdate::Status` 与 `UiUpdate::Finished`。

#### 验收标准
- `SessionRunner` 具备针对 MockStream 的独立单元测试；
- 协议调度流程标准化，不再随品牌增加而复制主循环。

---

### Phase 3: OPPO 品牌驱动垂直切片重构 (OPPO Driver Slicing)

#### 目标
将 2381 行的 [`src/bluetooth/brands/oppo/oppo_session.rs`](../src/bluetooth/brands/oppo/oppo_session.rs) 巨石单文件按垂直切片解耦，消灭局部影子状态。

#### 详细任务
1. **重构目录组织**：
   ```text
   src/bluetooth/brands/oppo/
   ├── mod.rs               # 导出 OppoDriver 与构造函数
   ├── driver.rs            # 实现 HeadsetDriver trait（仅调度，~150 行）
   ├── init.rs              # 握手、Capabilities 解析、组装 DeviceSnapshot
   ├── unsolicited.rs       # 处理 0x0100 等未请求事件与广播
   └── features/            # 垂直业务写管道（单一职责）
       ├── mod.rs
       ├── noise.rs         # 降噪指令、佩戴检查、ACK + 回读
       ├── sound.rs         # EQ 切换、自定义 EQ、空间音频、游戏模式、低音增强
       ├── controls.rs      # 手势配置、佩戴检测开关、提示音量
       └── dual_device.rs   # 双设备开关与配对列表读写
   ```
2. **消除 Worker 影子状态**：
   - 废除在 Worker 中克隆的 10 个局部变量；
   - 在 `OppoDriver` 中仅维护极简的运行时上下文（`read_only` 标记、能力位掩码与最新佩戴状态门控）；
3. **接入通用 `SessionRunner`**：
   - `spawn_oppo_monitor` 直接委托给通用引擎。

#### 验收标准
- `oppo_session.rs` 巨石文件被完全消除，单个业务切片文件控制在 200~300 行内；
- 现有关于 OPPO/OnePlus 的所有单元测试（写后等待 ACK、回读校验、佩戴门控等）100% 通过。

---

### Phase 4: EarFun 品牌驱动适配与驱动注册表 (EarFun & Driver Registry)

#### 目标
将 [`src/bluetooth/brands/earfun/earfun_session.rs`](../src/bluetooth/brands/earfun/earfun_session.rs)（1562 行）适配为标准驱动，统一品牌工厂分发。

#### 详细任务
1. **重构 EarFun 驱动为垂直切片结构**：
   - 依照 Phase 3 规范将 EarFun GAIA V3 会话代码拆解为 `EarFunDriver`，复用切片快照；
2. **引入统一驱动工厂与注册表**：
   - 在 [`src/bluetooth/mod.rs`](../src/bluetooth/mod.rs) 中实现驱动创建工厂：
     ```rust
     pub fn create_driver(brand: DeviceBrand) -> Box<dyn HeadsetDriver> {
         match brand {
             DeviceBrand::EarFun => Box::new(brands::earfun::EarFunDriver::new()),
             DeviceBrand::OnePlus | DeviceBrand::Oppo | DeviceBrand::Realme => {
                 Box::new(brands::oppo::OppoDriver::new())
             }
         }
     }
     ```
3. **全面清理老旧残留代码**：
   - 移除旧有的两套巨石单文件；
   - 保证外部调用接口（`spawn_monitor`、`probe`）的完全透明兼容。

#### 验收标准
- 全量自动化测试（110+ 项）通过；
- 蓝牙会话生命周期代码完全收敛，品牌代码仅包含协议编解码与特定切片写管道。

---

### Phase 5: 新品牌接入指南与代码模板 (New Brand SOP)

#### 目标
固化开发标准，为后续接入新品牌（如华为 FreeBuds、小米 Buds 等）提供标准化流程。

#### 标准接入流程（SOP）
1. **协议层实现**（[`src/protocol/brands/<brand>/`](../src/protocol/)）：
   - 编写纯粹的帧结构、数据包编解码器（Codec）与无状态 Parser；
2. **型号配置声明**（[`src/compatibility/brands/<brand>/`](../src/compatibility/)）：
   - 声明 Product ID、型号名称与能力树（降噪模式、EQ 预设等）；
3. **驱动实现**（[`src/bluetooth/brands/<brand>/`](../src/bluetooth/brands/)）：
   - 实现 `HeadsetDriver`，在 `init.rs` 中将握手数据填充为标准 `DeviceSnapshot`；
   - 在 `features/` 中编写特定命令的写入与回读验证；
4. **工厂注册**：
   - 在 `DeviceBrand` 枚举与 `create_driver` 工厂中加入新分支，即可自动获得 UI、桌面托盘、QuickSettings 及 `--probe` 命令行诊断的全部功能。

---

## 四、 架构安全与质量红线

重构全程必须无条件遵循 [`AGENTS.md`](../AGENTS.md) 与 [`docs/ARCHITECTURE_GUIDELINES.md`](ARCHITECTURE_GUIDELINES.md) 的安全约束：

1. **单指令写管道（Single Command Write Pipeline）绝对遵循**：
   - 每个切片的写操作必须严格遵循：“能力校验 $\to$ 发送单指令 $\to$ 等待设备 ACK $\to$ 验证回读状态”，严禁无序并发写入；
2. **未知设备只读保护**：
   - 未在兼容目录中注册的 Product ID 严格保持只读，禁止进入任何写管道；
3. **隐私绝对保护**：
   - 蓝牙物理地址（MAC 地址）与设备序列号（SN）严禁打印到终端日志或写入磁盘；
   - 临时使用的配对地址在会话结束时立即随驱动释放彻底清除；
4. **严禁固件操作**：
   - 严禁包含任何 OTA、固件刷写或低级擦写指令。

