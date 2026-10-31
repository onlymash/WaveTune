# WaveTune UI 组件重构方案：解耦庞大 Widget 结构体与分层封装指南

## 一、 背景与现状剖析

### 1.1 现状痛点
在 WaveTune 的 UI 表现层中，各导航页面普遍采用了集中式“控件包”结构体，例如：
- [`SoundWidgets`](../src/ui/pages/sound/mod.rs)：包含 35 个公共字段；
- [`HomeWidgets`](../src/ui/mod.rs)：包含 37 个公共字段；
- [`ControlsWidgets`](../src/ui/pages/controls/mod.rs)：包含 26 个公共字段。

这些结构体在快速原型阶段起到了聚合引用的作用，但随着功能扩展，已显现出严重的架构技术债：

1. **“上帝结构体”（God Object）与扁平过度**：
   单个结构体平铺了整个页面全部细粒度的控件句柄。例如 `SoundWidgets` 将预设 EQ 列表、自定义 EQ 列表、EQ 曲线编辑器、低音增强滑块、空间音频单选组、编解码器下拉框全部混在一层，缺乏逻辑边界。
2. **抽象层次倒错与多重职责耦合**：
   结构体内不仅包含视图控件（`ActionRow`, `Switch`, `Scale`），还杂糅了：
   - 防重入控制标志（`ReentrancyFlag`）
   - 业务数据缓存（`Rc<RefCell<Vec<CustomEqualizer>>>`）
   - 临时交互状态（`CheckButton` 引用、动态生成的 `Vec<Row>`）
3. **构建函数膨胀与样板代码爆炸**：
   页面构建函数（如 `build_sound_page` 达 300 行、`build_controls_page` 达 340 行）承担了从控件实例化、样式修饰、层次组装到庞大结构体字段填装的全部脏活，参数和返回值极度臃肿。
4. **封装泄露与脆弱的外部直接修改**：
   所有字段均为 `pub(crate)` 裸露，外部业务层和更新函数（如 `sound_state.rs`）直接遍历操作底层原生控件。如果调用方忘记使用对应的 `ReentrancyFlag`，极易引发 UI 事件风暴或死循环。

### 1.2 重构目标
- **卡片化分治（Card-based Sub-Widgets）**：按界面视觉与业务逻辑区块（如 `EqualizerCard`、`BassEngineCard` 等）拆分细粒度子组件结构体。
- **高内聚封装（Encapsulation & Facade）**：收敛控件直接暴露，将防重入保护、控件状态设置、可用性更新封装为内部领域方法。
- **纯粹的视图层**：将业务数据缓冲从 UI 结构体中剔除，UI 仅反映 `AppState`，实现真正的单向数据驱动。
- **平滑演进、零业务破坏**：每次变更保持 `cargo test --locked` 和 `cargo check --locked` 持续通过，不破坏已有协议控制与交互逻辑。

---

## 二、 目标架构蓝图

### 2.1 结构演进对比

```
【重构前：扁平庞大的上帝结构体】
SoundWidgets (35+ 字段)
├── sound_columns
├── eq_card / preset_list / preset_radios / first_radio
├── custom_preset_rows / custom_equalizers / custom_list / custom_editor / band_box ...
├── enhancement_card / hires_switch / codec_row / bass_switch / bass_slider ...
├── spatial_card / spatial_list / spatial_radios
└── eq_flag / spatial_flag / bass_switch_flag / hires_switch_flag / codec_flag ...

【重构后：树状高内聚的组件化结构】
SoundWidgets
├── columns: AdaptiveColumns
├── eq: EqualizerWidgets             ──> 负责预设 EQ 列表卡片展示与选中
├── custom_eq: CustomEqualizerWidgets──> 负责自定义 EQ 列表、新建、重命名、频段滑块编辑
├── bass: BassEngineWidgets          ──> 负责低音增强开关与强度滑块（内置 flag 保护）
├── spatial: SpatialAudioWidgets     ──> 负责空间音频模式单选组
└── enhancement: EnhancementWidgets  ──> 负责高清音频与编解码器下拉选项
```

### 2.2 典型子组件设计范式（以 BassEngineWidgets 为例）

```rust
/// 低音引擎子组件：自包含控件与防重入机制，外部只能通过方法更新
#[derive(Clone)]
pub(crate) struct BassEngineWidgets {
    container: adw::PreferencesRow,
    switch_row: adw::ActionRow,
    switch: Switch,
    slider_row: adw::PreferencesRow,
    slider: Scale,
    flag: ReentrancyFlag,
}

impl BassEngineWidgets {
    /// 设置可见性（根据耳机能力）
    pub fn set_supported(&self, supported: bool) {
        self.switch_row.set_visible(supported);
        self.slider_row.set_visible(supported);
    }

    /// 更新启用状态（内部自动处理防重入和控件敏感度）
    pub fn set_enabled(&self, enabled: bool) {
        self.flag.suppress(|| {
            self.switch.set_active(enabled);
            self.slider.set_sensitive(enabled);
        });
    }

    /// 更新低音强度等级
    pub fn set_level(&self, level: BassEngineLevel) {
        self.flag.suppress(|| {
            self.slider.set_value(level.into());
        });
    }

    /// 重置为初始失能状态
    pub fn reset(&self) {
        self.flag.suppress(|| {
            self.switch.set_active(false);
            self.switch.set_sensitive(false);
            self.slider.set_value(0.0);
            self.slider.set_sensitive(false);
        });
    }
}
```

---

## 三、 渐进式实施步骤（5 阶段演进）

### 阶段一：试点重构 —— `SoundWidgets` 卡片化拆分 [已完成]
**目标**：以当前代码最长、结构最典型的 `SoundWidgets` 为突破口，拆分出独立的子结构体与构建函数。

#### 详细任务：
1. **新建子模块**：在 `src/ui/pages/sound/` 下将功能拆分至专属子文件或分组：
   - `eq.rs`：`EqualizerWidgets`（预设列表）
   - `custom_eq.rs`：`CustomEqualizerWidgets`（自定义预设与编辑器）
   - `enhancement.rs`：`BassEngineWidgets` 与 `EnhancementWidgets`（Hi-Res / Codec）
   - `spatial.rs`：`SpatialAudioWidgets`（空间音频）
2. **拆解构建逻辑**：
   - 将原 300 行的 `build_sound_page()` 拆分为多个细粒度的卡片构建函数：
     - `build_eq_card() -> (adw::PreferencesGroup, EqualizerWidgets)`
     - `build_custom_eq_card(...) -> (adw::PreferencesGroup, CustomEqualizerWidgets)`
     - `build_enhancement_card() -> (adw::PreferencesGroup, BassEngineWidgets, EnhancementWidgets)`
     - `build_spatial_card() -> (adw::PreferencesGroup, SpatialAudioWidgets)`
   - 主函数 `build_sound_page()` 仅负责将各卡片挂入 `AdaptiveColumns` 布局。
3. **适配 `SoundPageFeaturesViewState` 与状态更新函数**：
   - 相应调整 `src/ui/pages/sound/state.rs` 中的更新函数，按子组件拆分调用参数。

* **验收标准**：
  - `src/ui/pages/sound/mod.rs` 行数减少 60% 以上；
  - `SoundWidgets` 主结构体仅包含 6 个子字段；
  - `cargo check --locked` 与 `cargo test --locked` 全部通过。

---

### 阶段二：封装下沉 —— 消除字段裸露与业务数据外置 [已完成]
**目标**：私有化子组件字段，消除 `Rc<RefCell<Vec<CustomEqualizer>>>` 等业务状态对 UI 的污染。

#### 详细任务：
1. **私有化控件字段**：
   - 将子组件（如 `BassEngineWidgets`、`SpatialAudioWidgets`）内的 `Switch`, `Scale`, `ReentrancyFlag` 改为私有字段。
   - 暴露语义化方法：`set_supported(...)`, `set_active(...)`, `bind_changed(...)`。
2. **外置自定义 EQ 业务数据**：
   - `custom_equalizers` 列表属于业务状态，应当存放在 `SoundState` 中，而不是作为 `Rc<RefCell<...>>` 挂在 UI 控件树上。
   - UI 仅接收 `&[CustomEqualizer]` 渲染列表项，编辑完成后的保存/删除通过 Action 事件向 Controller 派发。

* **验收标准**：
  - UI 结构体中不再存在 `Rc<RefCell<Vec<T>>>` 这类业务数据载荷；
  - 外部模块无法直接 bypass 防重入标志去设置控件。

---

### 阶段三：横向推广 —— `ControlsWidgets` 组件化拆分 [已完成]
**目标**：按照阶段一、二沉淀的模式，对 `ControlsWidgets`（手势配置与偏好设置）进行重构。

#### 详细任务：
1. **拆解为 4 个独立子组件**：
   - `FeatureTogglesWidgets`：双设备连接、偏好设备、佩戴检测、自动休眠、提示音量；
   - `HeadMotionWidgets`：头部动作控制开关与点头/摇头下拉框；
   - `GestureSettingsWidgets`：左右耳切换按钮、各手势触发下拉框、按键循环配置；
   - `UtilityWidgets`：查找耳机与贴合度测试。
2. **拆解 `build_controls_page`**：
   - 分离各个卡片的构建函数；
   - 消除 `gesture_combos: Vec<(GestureRow, adw::ComboRow)>` 的直接裸露，封装手势列表的更新与读取接口。
3. **重构 `src/ui/pages/controls/state.rs` 与 Controller/Component 交互**。

* **验收标准**：
  - `ControlsWidgets` 收敛为拥有 4 个子组件的高层结构体；
  - 控件字段全部私有化，通过领域语义方法与事件闭包通信；
  - 所有 111 项单元测试全部通过。

---

### 阶段四：横向推广 —— `HomeWidgets` 组件化拆分 [已完成]
**目标**：重构首页庞大的控件包，使耳机模型展示、电量、快捷降噪、快捷声音卡片各归其位。

#### 详细任务：
1. **拆解为 5 个独立子组件**：
   - `DeviceHeaderWidgets`：设备选择下拉菜单、设备名称、连接状态 Tag、刷新按钮；
   - `DeviceModelHeroWidgets`：3D 场景视图/图片转盘、固件版本、占位图切换；
   - `BatteryBannerWidgets`：左耳、右耳、充电盒电量指示器条；
   - `QuickNoiseWidgets`：降噪模式三态按钮组、高级降噪下拉框；
   - `QuickSoundWidgets`：EQ 快速切换、空间音频快速切换、游戏模式开关；
   - `PairedDevicesCardWidgets`：已配对设备列表卡片。
2. **重构 `HomeWidgets` 与 `src/ui/pages/home/state.rs`**：
   - 消除主结构体中的 37 个扁平字段；
   - 将转盘动画的内部状态（`turntable_state`, `current_frame`）进一步封装到 `DeviceModelHeroWidgets` 内部。

* **验收标准**：
  - `HomeWidgets` 字段从 37 个缩减至 6 个；
  - 首页各区块逻辑完全独立，各 Feature Component 仅持有自己需要的子组件。

---

### 阶段五：统一规约与测试验证
**目标**：建立 WaveTune UI 组件设计准则，确保后续新增品牌或特性时保持架构整洁。

#### 详细任务：
1. **编写 UI 组件开发规范**：在 `docs/` 或开发指南中明确：
   - 凡是包含多个相关控件的卡片必须提取为独立结构体；
   - 控件字段默认私有，通过 `pub fn` 提供带防重入保护的更新接口；
   - 页面主构建函数仅负责胶水装配，各卡片自建自管。
2. **全局质量验证**：
   - 运行完整测试套件：`cargo test --locked`
   - 运行格式与静态检查：`cargo fmt --check`、`cargo clippy --locked -- -D warnings`
   - 真机/交互回测验证：确保蓝牙连接、降噪切换、EQ 滑块、手势配置等所有 UI 交互行为与重构前严格一致。

---

## 四、 风险防范与安全底线

1. **GTK 信号重入风险**：
   重构将控件私有化时，必须严格保留原有的 `ReentrancyFlag::suppress` / `is_updating()` 防重入拦截，切勿在封装过程中漏掉标志位守卫导致事件死循环。
2. **生命周期与引用计数**：
   拆分子结构体时注意 Rust 所有权和 GObject 内部引用的配合，避免出现循环引用（`Rc` 环）导致组件无法被释放。
3. **保持协议层隔离**：
   遵循 `AGENTS.md` 规范，UI 组件内绝不渗透任何底层的 RFCOMM 报文、蓝牙原始地址或设备私有命令协议，所有交互严格经由 `Action` / `AppState` 转发。

