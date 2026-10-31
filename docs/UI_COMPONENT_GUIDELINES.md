# WaveTune UI 组件设计与开发规范

本文档为 WaveTune（基于 Rust + GTK4 / Libadwaita）制定 UI 组件化设计准则，指导后续新增页面、功能卡片或品牌定制组件时的开发，确保 UI 代码具备高内聚、低耦合、字段私有化以及完善的防重入机制。

---

## 一、 架构设计核心原则

1. **分层清晰与隔离**：
   - **Protocol 层**：纯协议编解码，严禁依赖 GTK 或 BlueZ。
   - **Feature Controller / Component 层**：负责业务逻辑、后台命令派发与状态聚合。
   - **UI Widgets 层**：纯视觉呈现与用户交互事件封装，严禁渗透底层 RFCOMM 报文、蓝牙原始字节或私有命令。

2. **卡片自建自管（Self-Contained Sub-Widgets）**：
   - 页面中每个逻辑卡片或相关控件组（如降噪设置、电量栏、模型展示、手势配置、均衡器等）必须提取为独立的子模块与子结构体；
   - 页面主模块（`mod.rs`）仅作为**胶水层（Glue Layer）**，负责布局装配与流式整合，禁止在页面主结构体中平铺数十个裸露的 GTK 控件字段。

3. **控件私有与语义化接口（Encapsulation & Semantic API）**：
   - 子组件结构体内部的所有 GTK 控件字段**默认私有**（不使用 `pub`）；
   - 外部通过具有明确领域语义的方法（如 `render()`、`update_*()`、`set_visible()`、`reset()`）进行状态更新；
   - 用户事件通过语义化回调（如 `connect_mode_selected`、`connect_switch_toggled`）对外暴露，由组件内部完成 GTK 信号绑定、选项 ID 解析与防重入拦截。

---

## 二、 目录与模块组织规范

每个页面在 `src/ui/pages/` 下拥有专属子目录，按功能拆分为独立文件：

```text
src/ui/pages/
├── home/
│   ├── mod.rs        # 页面聚合、主结构体 HomeWidgets、build_home_page()
│   ├── header.rs     # 设备选择下拉列表、连接状态 Tag、刷新按钮 (DeviceHeaderWidgets)
│   ├── battery.rs    # 左/右耳/充电盒电量指示器条 (BatteryBannerWidgets)
│   ├── hero.rs       # 3D VFX 场景 / 2D 转盘动画与模型卡片 (DeviceModelHeroWidgets)
│   ├── noise.rs      # 降噪模式三态按钮组与高级降噪选择 (QuickNoiseWidgets)
│   ├── sound.rs      # 快速 EQ、空间音频与游戏模式卡片 (QuickSoundWidgets)
│   ├── paired.rs     # 已配对设备列表卡片与管理交互 (PairedDevicesCardWidgets)
│   └── state.rs      # 视图状态模型（ViewState）与纯函数辅助逻辑
├── sound/
│   ├── mod.rs        # SoundWidgets、build_sound_page()
│   ├── effects.rs    # 低音引擎、空间音频、游戏模式 (SoundEffectsWidgets)
│   ├── presets.rs    # 均衡器预设列表与切换 (EqualizerPresetWidgets)
│   ├── custom_eq.rs  # 自定义 10 段 EQ 调节器 (CustomEqualizerWidgets)
│   └── state.rs
└── controls/
    ├── mod.rs        # ControlsWidgets、build_controls_page()
    ├── features.rs   # 双设备连接、佩戴检测、自动休眠 (FeatureTogglesWidgets)
    ├── head_motion.rs# 头部动作控制 (HeadMotionWidgets)
    ├── gestures.rs   # 耳机手势映射与长按循环 (GestureSettingsWidgets)
    ├── utilities.rs  # 查找耳机与贴合度测试 (UtilityWidgets)
    └── state.rs
```

---

## 三、 子组件结构体定义规范

每个子组件结构体应遵循如下模式：

```rust
use gtk::prelude::*;
use crate::ui::reentrancy::ReentrancyFlag;

#[derive(Clone)]
pub(crate) struct MyFeatureWidgets {
    // 1. 内部控件一律私有
    container: gtk::Box,
    switch: gtk::Switch,
    combo: adw::ComboRow,
    
    // 2. 状态变更可能由外部触发，需配备防重入标志
    switch_flag: ReentrancyFlag,
    combo_flag: ReentrancyFlag,
}

impl MyFeatureWidgets {
    // 3. 领域语义渲染/更新方法
    pub fn update(&self, state: &MyFeatureViewState) {
        let _guard = self.switch_flag.enter();
        self.switch.set_active(state.enabled);
        self.switch.set_sensitive(state.sensitive);
        
        let _guard = self.combo_flag.enter();
        // 渲染下拉选项...
    }

    pub fn reset(&self) {
        let _guard = self.switch_flag.enter();
        self.switch.set_active(false);
        self.switch.set_sensitive(false);
    }

    // 4. 事件连接方法：负责解包与防重入，对外只暴露领域类型（如 bool, u8, ID）
    pub fn connect_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let flag = self.switch_flag.clone();
        self.switch.connect_state_set(move |switch, state| {
            if flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            switch.set_sensitive(false);
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }
}

// 5. 独立的构建函数，返回 (根卡片容器, 组件句柄)
pub(crate) fn build_my_feature() -> (adw::PreferencesGroup, MyFeatureWidgets) {
    let group = adw::PreferencesGroup::new();
    // 构建控件层级...
    (group, widgets)
}

// 6. 测试专用检查器：置于 #[cfg(test)] 块中，杜绝生产代码滥用内部控件
#[cfg(test)]
#[allow(dead_code)]
impl MyFeatureWidgets {
    pub fn is_switch_active(&self) -> bool {
        self.switch.is_active()
    }
}
```

---

## 四、 信号防重入防护（Reentrancy Guard）规范

在双向绑定场景中（如蓝牙上报设备当前 EQ 为 1，UI 设置下拉框选中项 1，触发 `selected_notify` 导致错误发出重复写入指令），必须使用 `ReentrancyFlag`：

1. **更新入口加守卫**：
   在通过代码调用 `set_active()`、`set_selected()`、`set_value()` 前，必须获取守卫 RAII 句柄：
   ```rust
   let _guard = self.combo_flag.enter();
   self.combo.set_selected(target_idx);
   ```
2. **事件回调查状态**：
   在所有信号监听回调中，第一时间检查守卫状态，若正在编程更新则直接短路跳过：
   ```rust
   if flag.is_updating() {
       return;
   }
   ```

---

## 五、 页面构建函数（Glue Layer）规范

页面模块的主构建函数职责必须高度精炼：

1. 创建主视口容器（如 `page_container()` 或自适应双列流式布局容器 `AdaptiveColumns`）；
2. 调用各子组件的独立构建函数获取根组件和部件句柄；
3. 将根组件逐一挂载到页面视口中；
4. 构造页面级聚合结构体并返回 `(GtkBox, PageWidgets)`。

页面主构建函数的行数一般应控制在 **100 行以内**，禁止在其中包含控件细节样式配置、长列表项动态追加或复杂事件闭包。

---

## 六、 Feature Controller 消费规范

重构后的各功能 Controller（如 `NoiseController`、`SoundController`、`DeviceController`）应遵守：
1. **直接依赖子组件**：Controller 仅持有与其业务紧密相关的特定子组件（如 `QuickNoiseWidgets`、`EqualizerPresetWidgets`），不再持有大而全的页面结构体或克隆包装体；
2. **禁止反射/下钻深层控件**：Controller 不应尝试通过 `parent()` / `child()` 遍历 GTK 树或强转控件类型，所有界面操作必须经由子组件暴露的公共方法完成。

