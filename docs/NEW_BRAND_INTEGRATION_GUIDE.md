# WaveTune 新品牌耳机接入指南与开发标准 (New Brand SOP)

本文档为 WaveTune 跨品牌无线耳机控制架构的标准化接入指南（Standard Operating Procedure）。任何新耳机品牌（如华为 FreeBuds、小米 Buds、漫步者 Edifier、索尼 Sony 等）的接入均须遵循本文档所定义的模块划分、分层隔离准则与单指令安全写管道。

---

## 一、 整体接入架构概览

WaveTune 采用**品牌命名空间隔离**与**垂直切片架构**，新增一个耳机品牌涉及以下四个层次的实现：

```text
┌────────────────────────────────────────────────────────────────────────┐
│ 1. 协议编解码层 (Protocol Codec Layer)                                 │
│    src/protocol/brands/<brand>/                                        │
│    - 纯无状态帧结构编码/解码 (Frame & AppPacket)                       │
│    - 命令字枚举、数据载荷 Parser 与序列化函数                          │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 2. 型号与能力目录层 (Catalog & Compatibility Layer)                    │
│    src/compatibility/brands/<brand>/                                   │
│    - Product ID 与型号映射                                              │
│    - 静态能力声明 (DeviceProfile: 降噪树、EQ 预设、手势定义等)           │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 3. 蓝牙驱动实现层 (Bluetooth Driver Slice Layer)                       │
│    src/bluetooth/brands/<brand>/                                       │
│    - 实现 HeadsetDriver trait，由通用 SessionRunner 统一调度            │
│    - 业务功能切片 (features/noise, sound, controls, dual_device)        │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 4. 工厂分发与发现注册 (Registry & Discovery Integration)               │
│    src/bluetooth/mod.rs & src/bluetooth/discovery.rs                   │
│    - DeviceBrand 枚举扩展                                              │
│    - AnyDriver 枚举与 create_driver 静态工厂注册                       │
│    - BlueZ 广播过滤规则 (Vendor ID / Control UUID)                     │
└────────────────────────────────────────────────────────────────────────┘
```

> [!IMPORTANT]
> 接入新品牌时，**完全无需修改** UI 表现层（GTK4/Libadwaita）、MVI 核心状态层（`HeadsetSessionState`）、D-Bus 服务、托盘服务与 `SessionRunner` 线程执行器。只需按规范实现上述四层，新品牌即可自动获得全部桌面控制能力。

---

## 二、 步骤详解与代码模板

### 步骤 1：协议编解码层实现 (`src/protocol/brands/<brand>/`)

本层只关注二进制协议的封包、拆包与解析，**严禁引入任何 GTK、BlueZ 或 I/O 依赖**。

#### 1. 目录结构
```text
src/protocol/brands/<brand>/
├── mod.rs               # 导出公共协议类型
├── packet.rs            # 数据包结构定义、序列化与反序列化
├── frame.rs             # 帧同步头、转义处理与 FrameDecoder 增量解码器
├── commands.rs          # 命令字常量或枚举
└── parsers.rs           # 电量、固件、佩戴、降噪等响应包解析函数
```

#### 2. FrameDecoder 规范模板
耳机通过 RFCOMM Socket 传输的数据是流式字节，必须实现增量解析器以应对粘包与半包：

```rust
// src/protocol/brands/<brand>/frame.rs
#[derive(Default)]
pub struct CustomFrameDecoder {
    buffer: Vec<u8>,
}

impl CustomFrameDecoder {
    /// 接收流式字节，返回解析出的完整帧列表
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        
        while let Some((frame, consumed)) = Self::extract_next_frame(&self.buffer)? {
            frames.push(frame);
            self.buffer.drain(..consumed);
        }
        
        // 限制未处理缓冲区最大容量，防止恶意或异常数据导致内存泄漏
        if self.buffer.len() > 4096 {
            self.buffer.clear();
        }
        
        Ok(frames)
    }

    fn extract_next_frame(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>, String> {
        // 实现帧头定位、长度校验、校验和 (CRC/Checksum) 验证
        todo!()
    }
}
```

---

### 步骤 2：型号与能力目录层 (`src/compatibility/brands/<brand>/`)

声明该品牌受支持的具体型号与其功能开关。未在此声明的设备将作为未知型号，由系统强制锁定为**只读模式**以保护硬件安全。

#### 1. 目录结构
```text
src/compatibility/brands/<brand>/
├── mod.rs
└── catalog.rs           # 声明具体型号的 DeviceProfile
```

#### 2. DeviceProfile 声明模板
```rust
// src/compatibility/brands/<brand>/catalog.rs
use crate::compatibility::{DeviceFeatures, DeviceProfile, NoiseMode, NoiseModeKind};

pub const BRAND_MODEL_1_PID: u32 = 0x123456;

static MODEL_1_NOISE_MODES: &[NoiseMode] = &[
    NoiseMode { index: 0, kind: NoiseModeKind::Off, parent: None },
    NoiseMode { index: 1, kind: NoiseModeKind::NoiseCancelling, parent: None },
    NoiseMode { index: 2, kind: NoiseModeKind::Transparency, parent: None },
];

pub static BRAND_PROFILES: &[DeviceProfile] = &[
    DeviceProfile {
        product_id: BRAND_MODEL_1_PID,
        model: "Example Earbuds Pro",
        features: DeviceFeatures::NOISE_CONTROL
            .union(DeviceFeatures::EQUALIZER)
            .union(DeviceFeatures::WEAR_DETECTION)
            .union(DeviceFeatures::DUAL_DEVICE),
        noise_modes: MODEL_1_NOISE_MODES,
    },
];
```

并在 `src/compatibility/registry.rs` 的全局型号查询中加入新品牌列表。

---

### 步骤 3：蓝牙驱动实现层 (`src/bluetooth/brands/<brand>/`)

按垂直切片实现新品牌的 `HeadsetDriver`。

#### 1. 标准切片结构
```text
src/bluetooth/brands/<brand>/
├── mod.rs               # 导出 BrandDriver 及构造方法
├── driver.rs            # 实现 HeadsetDriver trait（仅调度与状态持有）
├── init.rs              # 握手、版本读取、组装初始 DeviceSnapshot
├── transport.rs         # RFCOMM 连接建立、Candidate UUIDs、单指令 request()
├── unsolicited.rs       # 处理设备主动广播通知（如开盖、出入耳、电量广播）
└── features/            # 垂直功能写管道（单一职责）
    ├── mod.rs
    ├── noise.rs         # 降噪写指令、佩戴检查门控、回读确认
    ├── sound.rs         # EQ 调节、游戏模式、编解码器配置
    ├── controls.rs      # 手势定制、入耳检测开关
    └── dual_device.rs   # 双设备多点连接管理与设备列表读写
```

#### 2. HeadsetDriver 驱动实现模板
```rust
// src/bluetooth/brands/<brand>/driver.rs
use std::{fs::File, io::{Read, Write}, sync::mpsc::Sender, time::Duration};
use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, DeviceSnapshot, driver::HeadsetDriver},
};

pub struct BrandDriver {
    decoder: CustomFrameDecoder,
    snapshot: Option<DeviceSnapshot>,
    // 仅保存必要的运行时驱动状态（如佩戴状态门控、子模式缓存）
}

impl BrandDriver {
    pub fn new() -> Self {
        Self {
            decoder: CustomFrameDecoder::default(),
            snapshot: None,
        }
    }
}

impl HeadsetDriver for BrandDriver {
    fn brand_name(&self) -> &'static str {
        "brand_name"
    }

    fn open_rfcomm(&mut self, device: &DeviceChoice) -> std::io::Result<File> {
        super::transport::open_brand_rfcomm(device)
    }

    fn initialize<T: Read + Write>(
        &mut self,
        stream: &mut T,
        device: &DeviceChoice,
        updates: Option<&Sender<UiUpdate>>,
    ) -> Result<DeviceSnapshot, String> {
        let (snapshot, decoder) = super::init::initialize_brand_session(stream, device, updates)?;
        self.decoder = decoder;
        self.snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }

    fn handle_command<T: Read + Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        // 委托给对应的 features::* 切片处理，每个切片遵循单一指令写管道
        match command {
            WorkerCommand::SetNoiseMode(idx) => {
                super::features::noise::handle_set_noise_mode(stream, &mut self.decoder, idx, updates)
            }
            WorkerCommand::SetEqualizer(preset) => {
                super::features::sound::handle_set_equalizer(stream, &mut self.decoder, preset, updates)
            }
            // 其它指令...
            _ => Ok(()),
        }
    }

    fn handle_incoming_bytes<T: Read + Write>(
        &mut self,
        stream: &mut T,
        bytes: &[u8],
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let frames = self.decoder.push(bytes).map_err(|e| e.to_string())?;
        for frame in frames {
            super::unsolicited::handle_unsolicited(stream, &frame, updates);
        }
        Ok(())
    }

    // 可选：如果耳机协议不支持广播电量，可设置周期性轮询
    fn poll_interval(&self) -> Option<Duration> {
        None
    }
}
```

---

### 步骤 4：工厂分发与发现注册

#### 1. 扩展 `DeviceBrand` 枚举
在 [`src/protocol/core/types.rs`](file:///home/onlymash/Projects/WaveTune/src/protocol/core/types.rs) 中添加新品牌枚举变体：
```rust
pub enum DeviceBrand {
    OnePlus,
    Oppo,
    Realme,
    EarFun,
    NewBrand, // 新增
}
```

#### 2. 在驱动分发中心注册
在 [`src/bluetooth/mod.rs`](file:///home/onlymash/Projects/WaveTune/src/bluetooth/mod.rs) 中的 `AnyDriver` 枚举与 `create_driver` 工厂中加入分支：

```rust
pub enum AnyDriver {
    EarFun(brands::earfun::EarFunDriver),
    Oppo(brands::oppo::OppoDriver),
    NewBrand(brands::new_brand::NewBrandDriver), // 新增变体并转发 Trait 方法
}

pub fn create_driver(brand: DeviceBrand) -> AnyDriver {
    match brand {
        DeviceBrand::EarFun => AnyDriver::EarFun(brands::earfun::EarFunDriver::new()),
        DeviceBrand::NewBrand => AnyDriver::NewBrand(brands::new_brand::NewBrandDriver::new()),
        DeviceBrand::OnePlus | DeviceBrand::Oppo | DeviceBrand::Realme => {
            AnyDriver::Oppo(brands::oppo::OppoDriver::new())
        }
    }
}
```

#### 3. 注册 BlueZ 发现规则
在 [`src/bluetooth/discovery.rs`](file:///home/onlymash/Projects/WaveTune/src/bluetooth/discovery.rs) 中：
- 注册该品牌的 Bluetooth SIG Company Identifier（厂商 ID）或专有控制服务 UUID；
- 确保系统启动或蓝牙扫描时能自动识别该品牌耳机并指派对应的 `DeviceBrand`。

---

## 三、 安全红线与开发质量检查清单

在提交新品牌代码合并前，必须逐项核对并满足以下安全红线：

- [ ] **单指令写管道（Single Command Write Pipeline）**：
  - 严禁并发或连续无间隔下发写指令；
  - 写入流程必须严格为：`校验能力 -> 发送指令 -> 等待硬件 ACK -> 发送读取指令回读验证`；
  - 若回读状态不匹配或超时，必须向 UI 抛出 `UiUpdate::SettingError`，维持界面原有确认状态。
- [ ] **未知设备只读隔离**：
  - 未在 `catalog.rs` 注册的 Product ID，其快照必须设置 `read_only: true`；
  - 只读设备阻断所有写指令，仅允许查询或被动接收电量/固件信息。
- [ ] **隐私保护与无残留**：
  - 终端控制台、日志（`println!`, `eprintln!`, `tracing`）中**严禁打印物理 MAC 地址、设备序列号（SN）**；
  - 双设备管理中所读取的配对设备 MAC，仅在内存快照中供用户界面操作使用，会话断开后立即丢弃，严禁落盘。
- [ ] **严禁任何固件写操作**：
  - 绝对严禁实现固件升级（OTA）、擦写存储器或未公开工程调试命令。
- [ ] **国际化覆盖**：
  - UI 抛出的状态提示必须在 `locales/` 的 Fluent 文件中定义，控制台调试信息保持纯英文。
- [ ] **测试覆盖与代码格式**：
  - 针对该协议编写完整的 MockStream 单元测试；
  - 确保 `cargo fmt --check`、`cargo check --locked` 与 `cargo test --locked` 100% 绿色通过。

