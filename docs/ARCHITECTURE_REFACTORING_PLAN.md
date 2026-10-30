# WaveTune 架构重构方案：轻量 MVI 分层与垂直切片演进指南

## 一、 背景与现状剖析

### 1.1 现状痛点
WaveTune 最初基于 GTK4/Libadwaita 与 BlueZ 开发，由于初期缺乏严格的架构分层规范，随着支持的耳机特性（降噪、多模式EQ、空间音频、手势控制、双设备流转等）不断扩充，代码逐渐演进出以下典型架构技术债：

1. **“上帝对象”失控**：`src/app/controller.rs` 膨胀至 3000 余行，集成了设备发现、通道生命周期、全部 UI 信号绑定、全部后台事件处理、状态暂存以及 D-Bus/托盘同步，职责严重过载。
2. **状态碎片化**：控制器内部维护了 30 余个独立的 `Rc<Cell<T>>` 和 `Rc<RefCell<T>>`，状态缺乏统一视图，且容易在多处产生状态不一致。
3. **函数参数爆炸**：UI 渲染函数需要逐个接收离散的状态字段与控件引用，大量函数参数达到 8～10 个（例如 `update_noise_controls`），不得不依靠 `#[allow(clippy::too_many_arguments)]` 压制 linter 警告。
4. **防重入标志泛滥**：为了阻止 GTK4 控件在程序设置值时反向触发信号形成事件风暴，代码中散落着大量 `updating_xxx: &Cell<bool>` 标志位，由外层逐级透传进渲染函数，不仅极易遗漏复位，也严重污染了函数签名。
5. **事件循环定时轮询**：主线程使用 `glib::timeout_add_local(100ms)` 定时循环调用 `try_recv()`，既存在 100ms 的事件处理延迟，又在空闲时白白浪费 CPU。

### 1.2 重构目标
将现有架构逐步平滑重构成**基于垂直切片（Feature-First）的轻量 MVI（Model-View-Intent）单向数据流架构**：
- **单一可信状态源（Single Source of Truth）**：所有业务与 UI 状态聚合到统一的领域结构体中。
- **单向数据流（UDF）**：用户交互产生 `Action/Intent` $\to$ 后台 Worker 执行 $\to$ 派发新状态/事件 $\to$ 集中更新 State $\to$ 驱动 UI 组件执行 `render(&State)`。
- **高内聚垂直切片**：拆分 3000 行单体控制器，按业务领域独立组织（Noise、Sound、Device、Controls 等）。
- **零成本 RAII 防重入**：引入守卫机制消除散落的 `Cell<bool>` 参数。
- **原生事件驱动**：用 `glib::MainContext::channel` 彻底替换 100ms 轮询。

---

## 二、 目标架构蓝图

```
┌────────────────────────────────────────────────────────────────────────┐
│                        UI 表现层 (GTK4 / Libadwaita)                    │
│                                                                        │
│   HomeView          SoundView         ControlsView       DeviceView    │
│      │                  │                  │                  │        │
│   ┌──┴──────────────────┴──────────────────┴──────────────────┴────┐   │
│   │                      Feature Components                        │   │
│   │  (NoiseComponent, EqualizerComponent, DeviceListComponent...)   │   │
│   │                                                                │   │
│   │  - 拥有相关 GTK 控件与局部防重入 Guard                         │   │
│   │  - 实现纯净的 fn render(&self, &DomainState) 视图映射          │   │
│   │  - 捕获用户操作并转换为 Action/Intent 发往 Dispatcher          │   │
│   └──────────────────────────────┬─────────────────────────────────┘   │
└──────────────────────────────────┼─────────────────────────────────────┘
                                   │ 1. User Intent (Action)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                         业务与状态分发层 (MVI Core)                    │
│                                                                        │
│   ┌────────────────────────────────────────────────────────────────┐   │
│   │                       Dispatcher / Store                       │   │
│   │                                                                │   │
│   │  - 状态管理：拥有单一集中状态 HeadsetSessionState              │   │
│   │  - 状态归约：收到后台 Update 或用户 Intent 时执行 Reducer     │   │
│   │  - 变更派发：状态变更后通知对应 FeatureComponent.render()       │   │
│   │  - 广播外设：生成快照同步至 D-Bus / QuickSettings / Tray       │   │
│   └──────────────▲─────────────────────────────────┬───────────────┘   │
│                  │                                 │                   │
│                  │ 4. UiUpdate (glib::channel)     │ 2. WorkerCommand  │
└──────────────────┼─────────────────────────────────┼───────────────────┘
                   │                                 ▼
┌──────────────────┴─────────────────────────────────────────────────────┐
│                       底层通信与基础设施层 (IO Workers)                │
│                                                                        │
│   BlueZ Discovery      RFCOMM Worker (Private Protocol)     D-Bus      │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 三、 渐进式实施方案（5 阶段演进路线）

为保证项目始终处于可编译、可测试、功能正常的状态，重构严格按照**“不破坏现有运行逻辑，逐步替换内部实现”**的原则推进。

### 阶段一：参数对象化与渲染函数去味（消除代码异味）[已完成]
**目标**：不改动现有状态流动和生命周期，先解决函数传参过多问题，移除 `#[allow(clippy::too_many_arguments)]`。

#### 具体任务：
1. **定义各页面渲染专用的 ViewState 结构体**（采用纯引用或切片，无堆内存拷贝）：
   * 在 `src/ui/pages/home/state.rs` 中定义 `NoiseControlsViewState<'a>`。
   * 在 `src/ui/pages/sound/state.rs` 中定义 `EqualizerViewState<'a>`、`SoundFeaturesViewState<'a>`。
   * 在 `src/ui/pages/controls/state.rs` 中定义 `GestureViewState<'a>`。
2. **重构函数签名**：
   * 将 `update_noise_controls(widgets, indices, note, enabled, wear_supports, wear, mode, modes, flag)` 简化为：
     ```rust
     pub(crate) fn update_noise_controls(
         widgets: &HomeWidgetRefs,
         state: &NoiseControlsViewState,
         updating_option_row: &Cell<bool>,
     )
     ```
3. **适配 `controller.rs` 中的调用点**，消除 `clippy::too_many_arguments`。

* **验收标准**：`cargo clippy` 在该模块无警告，所有单元测试通过。

---

### 阶段二：原生事件驱动改造（消除 100ms 轮询）[已完成]
**目标**：移除低效的 `timeout_add_local(100ms)` 轮询，改用异步通道与主上下文唤醒，提升响应速度并降低空闲能耗。

#### 具体任务：
1. 建立 `wavetune-ui-bridge` 线程与 Tokio/GLib 异步通道，消除定时轮询。
2. 移除 `gtk::glib::timeout_add_local(Duration::from_millis(100), ...)`。
3. 使用 `gtk::glib::spawn_future_local` 直接就地响应后台到达的 `UiUpdate` 消息。

* **验收标准**：UI 更新完全由消息到达即时唤醒驱动，CPU 占用更低，测试运行正常。

---

### 阶段三：状态聚合与单一数据源（State Unification）[已完成]
**目标**：将 `controller.rs` 内部散落的 30 多个 `Rc<Cell<T>>` 统一聚合成领域状态树。

#### 具体任务：
1. **新建 `src/app/state.rs`**，定义结构化状态：
   ```rust
   #[derive(Default, Clone, Debug)]
   pub struct HeadsetSessionState {
       pub connected: bool,
       pub device_name: String,
       pub battery: BatteryState,
       pub wear: WearState,
       pub noise: NoiseState,
       pub sound: SoundState,
       pub controls: ControlsState,
       pub dual_device: DualDeviceState,
   }

   #[derive(Default, Clone, Debug)]
   pub struct NoiseState {
       pub enabled: bool,
       pub current_mode: Option<u8>,
       pub supported_indices: Vec<u8>,
       pub modes: Vec<NoiseMode>,
       pub support_note: Option<String>,
   }

   #[derive(Default, Clone, Debug)]
   pub struct SoundState {
       pub current_preset: Option<u8>,
       pub presets: Vec<EqualizerPreset>,
       pub custom_equalizers: Vec<CustomEqualizer>,
       pub spatial_mode: Option<u8>,
       pub game_mode: Option<bool>,
       pub bass_enabled: Option<bool>,
       pub bass_level: Option<BassEngineLevel>,
       pub hi_res_enabled: Option<bool>,
       pub audio_codec: Option<u8>,
       // 能力标志统一归入状态
       pub supports_eq: bool,
       pub supports_spatial: bool,
       pub supports_game_mode: bool,
       pub supports_bass: bool,
       pub supports_hi_res: bool,
       pub supports_codec: bool,
   }
   ```
2. **在 `controller.rs` 中以 `Rc<RefCell<HeadsetSessionState>>` 替换离散变量**：
   * 后台收到 `UiUpdate` 时，直接在 `state.borrow_mut()` 上就地更新（Reducer 思路）。
   * `sync_desktop_services` 直接读取 `state` 生成快照，不再需要繁琐的提取操作。

* **验收标准**：`controller.rs` 内部局部变量减少 80%，状态具备全局快照能力。

---

### 阶段四：引入 RAII 防重入守卫（封装 Component）[已完成]
**目标**：彻底消除散布在各处的 `updating_xxx: &Cell<bool>` 标志位。

#### 具体任务：
1. **实现通用的 `ReentrancyGuard`（防重入守卫）**：
   ```rust
   pub struct UpdatingGuard<'a>(&'a Cell<bool>);
   impl<'a> Drop for UpdatingGuard<'a> {
       fn drop(&mut self) {
           self.0.set(false);
       }
   }

   #[derive(Default)]
   pub struct ReentrancyFlag(Cell<bool>);
   impl ReentrancyFlag {
       pub fn is_updating(&self) -> bool { self.0.get() }
       pub fn begin_update(&self) -> Option<UpdatingGuard> {
           if self.0.get() { return None; }
           self.0.set(true);
           Some(UpdatingGuard(&self.0))
       }
   }
   ```
2. **封装 UI Feature 视图组件（如 `NoiseComponent`）**：
   * 将控件集合与防重入标志打包。
   * 提供方法 `fn render(&self, state: &NoiseState)`：
     在函数入口获取 `_guard = self.flag.begin_update()`，更新控件，退出作用域自动复位。
   * 信号回调函数内检查 `if self.flag.is_updating() { return; }`。

* **验收标准**：UI 更新函数签名中不再有任何 `&Cell<bool>`。

---

### 阶段五：控制器垂直切片拆解（消解 3000 行上帝文件）
**目标**：将 `src/app/controller.rs` 拆解为垂直特性控制器，主入口只保留组装与生命周期路由，代码量缩减至 200 行以内。

#### 建议模块结构：
```text
src/app/
├── state.rs                     // 全局/领域状态模型
├── controller.rs                // 主入口（组装各 Feature Controller，处理整体生命周期）
├── events.rs                    // 保持现有的 WorkerCommand 和 UiUpdate 定义
├── desktop/                     // 桌面集成服务 (D-Bus, QuickSettings, Tray)
└── features/                    // 垂直业务切片
    ├── noise/
    │   ├── component.rs         // NoiseComponent (UI 控件与 render)
    │   └── controller.rs        // 按钮点击事件处理、指令派发与状态响应
    ├── sound/
    │   ├── component.rs         // EQ、空间音频、低音引擎等控件
    │   └── controller.rs
    ├── controls/
    │   ├── component.rs         // 手势配置控件
    │   └── controller.rs
    └── device/
        ├── component.rs         // 配对设备列表、固件卡片、电量条
        └── controller.rs
```

#### 各 Feature Controller 的标准职责：
每个 Feature Controller 仅暴露两个核心方法：
1. `pub fn bind_events(&self, action_sender: ActionSender)`：负责将 UI 信号转换为强类型的 Action 抛出。
2. `pub fn on_state_changed(&self, state: &DomainState)`：根据自身负责的领域状态，调用 `component.render(state)`。

* **验收标准**：
  * `controller.rs` 核心文件行数在 200 行以内。
  * 各个 Feature 职责高度内聚、代码结构对称，新增功能时只需在对应 Feature 目录下扩展。

---

## 四、 质量保障与安全准则

重构过程中必须严格遵循 `AGENTS.md` 所规定的蓝牙与隐私安全规范：
1. **测试驱动安全验证**：每一步修改后，必须运行 `cargo test --locked` 保证现有的协议帧编解码和设备配置解析不被破坏。
2. **隐私规范保留**：蓝牙设备物理地址、序列号等敏感数据严禁持久化，在内存状态（`HeadsetSessionState`）中仅在必要时临时保留，禁止通过 debug 或日志打印原始地址。
3. **写操作安全性**：重构事件绑定和 Action 分发逻辑时，必须保留原有的一次只发一条指令、必须等待设备 ACK 与权威回读状态确认的约束。

