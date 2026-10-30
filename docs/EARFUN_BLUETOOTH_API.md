# EarFun Wireless Headset Control Protocol (GAIA V3 Variant)

This document describes the private control protocol used by EarFun wireless headphones and earbuds (such as the **EarFun Air Pro 4**, **EarFun Air S**, and **EarFun Free Pro 3**).

The protocol is a variant of Qualcomm GAIA (Generic Access Profile Interface Architecture) v3, operating over a standard Bluetooth RFCOMM (Serial Port Profile) link without checksumming.

Reference implementations and hardware validation:
- **Client Reference**: Freeyourgadget / Gadgetbridge (PR #4635 / commit `d9d0573812`, `cab12149d6`, `1473364031`).
- **Hardware Validation**: Tested live on real **EarFun Air Pro 4** (Hardware/Firmware `TW600-3091_20250919_v0.4.1`, Product ID `45 35`, connected to Arch Linux via RFCOMM channel 11).

---

## 1. Bluetooth Connection and Discovery

### 1.1 Bluetooth Identity and SDP Records
EarFun devices advertise standard Bluetooth Classic audio and serial profiles:

- **Device Name**: `EarFun Air Pro 4`, `EarFun Air S`, `EarFun Free Pro 3`, etc.
- **Bluetooth Class**: `0x00240404` (Audio / Headset)
- **Manufacturer Vendor ID**: `0x5A70` (`23152`).
  - Manufacturer Specific Data in BLE/EIR advertisement carries `[0x70, 0x5A, ...]`.
  - Format includes the lower 4 bytes of the device MAC address followed by the 2-byte **Product ID**.
  - **Product IDs**:
    - **EarFun Air Pro 4**: `[0x45, 0x35]` (`0x004535`, ASCII `"E5"`)
    - **EarFun Free Pro 3**: `[0x45, 0x34]` (`0x004534`, ASCII `"E4"`)
    - **EarFun Air S**: `[0x4C, 0x45]` (`0x004C45`, ASCII `"LE"`)
- **Service UUIDs**:
  - Standard SPP UUID: `00001101-0000-1000-8000-00805F9B34FB`
  - Qualcomm GAIA UUID: `00001107-D102-11E1-9B23-00025B00A5A5`
  - Vendor Specific UUIDs: `0000eb04-d102-11e1-9b23-00025b00a5a5` ... `eb07`
- **RFCOMM Channel**:
  - Live probe on EarFun Air Pro 4 confirms that **RFCOMM Channel 11** accepts the GAIA control connection.
  - (Channel 22 is also exposed for raw diagnostic/telemetry streams).

### 1.2 Product ID and Model Identification Architecture
WaveTune strictly enforces that Bluetooth device names and user aliases are display-only and never used as a logic filter or product discriminator.

EarFun hardware identification resolves through two objective hardware-level mechanisms:
1. **Bluetooth Manufacturer Specific Data (`0x5A70`)**:
   During BlueZ discovery, if the device advertises `0x5A70`, the Product ID is extracted by matching the device MAC address suffix within the data payload:
   - `0x45, 0x35` $\to$ Product ID `0x004535` (EarFun Air Pro 4)
   - `0x45, 0x34` $\to$ Product ID `0x004534` (EarFun Free Pro 3)
   - `0x4C, 0x45` $\to$ Product ID `0x004C45` (EarFun Air S)
2. **RFCOMM Firmware Hardware Project Code (`0x0309`)**:
   When connecting over Bluetooth Classic without retained BLE advertising data, querying `0x0309` returns the vendor firmware string:
   - Starts with `TW600` (e.g., `TW600-3091_...`) $\to$ Product ID `0x004535` (Air Pro 4)
   - Starts with `TW400` / `TW400B` $\to$ Product ID `0x004534` (Free Pro 3)
   - Starts with `TW201` $\to$ Product ID `0x004C45` (Air S)
3. **Safety Fallback**:
   If neither source identifies a recognized Product ID, the session enters safe **Read-Only Mode** (`is_known_model: false`, `read_only: true`), displaying `Unknown Model` and disabling all control writes.

---

## 2. Frame Architecture

EarFun frames use the Qualcomm GAIA V3 frame layout without a checksum.

### 2.1 Packet Format

| Offset (Bytes) | Field | Value / Meaning |
|---|---|---|
| `0` | **SOP** | Start of Packet: constant `0xFF` |
| `1` | **Version** | Protocol version: `0x04` (request default) or `0x03` (earbuds response) |
| `2` | **Flags** | Bit 0: Checksum flag. Always `0x00` (checksum is not used) |
| `3` | **Length** | Payload byte length $N$ ($0 \le N \le 255$) |
| `4..5` | **Vendor ID** | Big-Endian 16-bit Vendor Identifier:<br>• `0x000A` (Qualcomm GAIA v1/v2 legacy / standard vendor)<br>• `0x001D` (Qualcomm GAIA v3 vendor) |
| `6..7` | **Command ID** | Big-Endian 16-bit Command Identifier |
| `8..8+N-1` | **Payload** | Command-specific payload |

### 2.2 Command ID Structure
Command IDs are 16-bit words formatted as:
```text
Bit 15      : ACK / Response flag (0x8000)
Bit 14..8   : Command Type (0x01: Config, 0x02: Control, 0x03: Status, 0x0E/0x0F: EQ, 0x40: Notification)
Bit 7..0    : Command Code
```

- When the host sends a request or setting command (e.g., `0x0306`), the earbuds respond with `Command ID | 0x8000` (e.g., `0x8306`).
- In EarFun responses, the first payload byte is typically a **Status Code** (`0x00` = Success / OK), followed by the reported value(s).

---

## 3. Session Initialization Sequence

When connecting over RFCOMM Channel 11, the official client and Gadgetbridge perform the following handshake:

1. **Query Firmware Version**:
   - Host sends: `FF 04 00 00 00 0A 03 09`
   - Earbuds reply: `FF 03 00 1B 00 0A 83 09 [00] [UTF-8 String]`
   - Example response payload: `00 54 57 36 30 30 2D 33 30 39 31 5F 32 30 32 35 30 39 31 39 5F 76 30 2E 34 2E 31` (`"TW600-3091_20250919_v0.4.1"`).
2. **Query Battery Levels**:
   - Left Earbud: `0x0306`
   - Right Earbud: `0x0307`
   - Charging Case: `0x0317`
3. **Query Audio Experience / Sound Controls**:
   - Current Noise Control (Ambient Sound) Mode: `0x0315`
   - Active Noise Cancelling (ANC) Sub-Mode: `0x033B`
   - Transparency Sub-Mode: `0x033D`
   - Game Mode (Low Latency): `0x0313`
4. **Query System & Connectivity Settings**:
   - In-Ear Detection: `0x034A`
   - Dual Device Connection (Multipoint): `0x0326`
   - Touch Controls / Gestures: `0x030B`
   - Touch Lock Mode: `0x0335`
   - Audio Codec: `0x032F`
   - Voice Prompt Volume: `0x0339`
   - Microphone Mode: `0x0331`
   - Find Device: `0x0333`

---

## 4. Commands and Payload Reference

### 4.1 Battery Status
| Feature | Command ID | Request Payload | Response ID | Response Payload | Description |
|---|---|---|---|---|---|
| Left Battery | `0x0306` | Empty (`[]`) | `0x8306` | `[00, level]` | `level`: 0–100 percentage |
| Right Battery | `0x0307` | Empty (`[]`) | `0x8307` | `[00, level]` | `level`: 0–100 percentage |
| Case Battery | `0x0317` | Empty (`[]`) | `0x8317` | `[00, level]` | `level`: 0–100 percentage (not supported on EarFun Air S) |

*Notes on Model Differences*:
- **EarFun Air Pro 4 & Free Pro 3**: Support 3 battery readings (Left, Right, Case).
- **EarFun Air S**: Only reports Left and Right battery levels (`0x0306`, `0x0307`). Querying Case Battery (`0x0317`) returns an error or times out, so case query is omitted for Air S.

*Example (live tested)*:
- Request Left: `FF 04 00 00 00 0A 03 06` $\to$ Response: `FF 03 00 02 00 0A 83 06 00 5A` (90%)
- Request Right: `FF 04 00 00 00 0A 03 07` $\to$ Response: `FF 03 00 02 00 0A 83 07 00 64` (100%)
- Request Case: `FF 04 00 00 00 0A 03 17` $\to$ Response: `FF 03 00 02 00 0A 83 17 00 64` (100%)

---

### 4.2 Noise Control (Ambient Sound & ANC)

EarFun headsets handle noise control differently depending on model generation:
- **EarFun Air Pro 4 & Free Pro 3**: Split into **Master Ambient Mode** (`0x0314` / `0x0315`) and **Sub-Modes** (`0x033A`/`0x033B` for ANC, `0x033C`/`0x033D` for Transparency).
- **EarFun Air S**: Directly sets and queries Master Ambient Mode (`0x0314` / `0x0315`) with 3 flat states (`0` = Off, `1` = ANC, `2` = Transparency) without secondary sub-modes.

#### Master Mode (`0x0314` Set / `0x0315` Query)
- **Query**: `0x0315` (Response `0x8315 [00, mode]`)
- **Set**: `0x0314 [mode]` (Response `0x8314 [00]`)
- **Modes**:
  - `0`: **Normal (Off)**
  - `1`: **Noise Cancelling (ANC)**
  - `2`: **Ambient Sound (Transparency)**

#### ANC Sub-Modes (`0x033A` Set / `0x033B` Query)
When Master Mode is `1` (ANC), the active ANC algorithm is configured via `0x033A`:
- **Query**: `0x033B` (Response `0x833B [00, submode]`)
- **Set**: `0x033A [submode]`
- **Submodes**:
  - `0`: **AI Ear Adaptive**
  - `1`: **Wind Noise Cancelling**
  - `2`: **Strong ANC**
  - `3`: **Balanced ANC**
  - `4`: **AI Environment Adaptive**

#### Transparency Sub-Modes (`0x033C` Set / `0x033D` Query)
When Master Mode is `2` (Transparency):
- **Query**: `0x033D` (Response `0x833D [00, submode]`)
- **Set**: `0x033C [submode]`
- **Submodes**:
  - `0`: **Default**
  - `1`: **Natural**

---

### 4.3 Low Latency / Game Mode
- **Query**: `0x0313` $\to$ Response `0x8313 [00, 00|01]`
- **Set**: `0x0312 [00|01]` $\to$ Response `0x8312 [00]`
- Values: `0` = Disabled, `1` = Enabled.

---

### 4.4 In-Ear Detection (Wear State)
- **Supported Models**: EarFun Air Pro 4 (Hardware-verified). EarFun Free Pro 3 and EarFun Air S do not have wear sensors and skip wear detection.
- **Query**: `0x034A` $\to$ Response `0x834A [00, val]`
- **Set**: `0x0349 [val]` $\to$ Response `0x8349 [00]`
- Values: `0` = In-Ear Detection Enabled, `1` = Disabled.

---

### 4.5 Dual Device Connection (Multipoint)
- **Query**: `0x0326` $\to$ Response `0x8326 [00, 00|01]`
- **Set**: `0x0327 [00|01]`
  - Note: Setting dual-device mode usually requires a reboot packet `0x0308` to take effect.
- Values: `0` = Disabled, `1` = Enabled.

---

### 4.6 Graphic Equalizer (10-Band vs 6-Band)
EarFun headsets support hardware equalization using Qualcomm GAIA V3 non-standard extensions (`Vendor ID = 0x001D`).

- **Write Command**: `0x0E01` (Set Band Gain)
- **Response**: `0x0F81 [01]` (ACK returned for each band written)
- **Query Command (`0x0E02`) Status**: **Not Supported (`COMMAND_NOT_SUPPORTED`)**
- **Gain Encoding**:
  - GAIA uses a factor of 60 for 1 dB. EarFun maps $\pm 6 \text{ dB}$ over a slider scale of $\pm 10$.
  - Formula: $\text{gain\_raw} = \text{round}(\text{gain\_val} \times 60 \times 0.6) = \text{round}(\text{gain\_val} \times 36)$.
- **Payload Format (9 bytes per band)**:
  ```text
  Byte 0   : Band ID (0xA1 .. 0xAA)
  Byte 1..2: Filter Type / Config (0xFF, 0x98)
  Byte 3..4: Frequency in Hz (16-bit Big-Endian)
  Byte 5..6: Gain (16-bit signed Big-Endian)
  Byte 7..8: Q-Factor (16-bit Big-Endian)
  ```

#### Write-Only Characteristics & Architecture Differences
1. **No Hardware Preset Concept**:
   - Unlike OPPO/OnePlus headsets which feature a firmware-level preset manager queried via `0x010F` (`CMD_GET_EQUALIZER`), EarFun headsets operate at the raw Qualcomm Kymera DSP PEQ (Parametric Equalizer) filter layer.
   - The headset firmware does not know what "Classic" or "Bass Boost" is; it only receives and stores raw DSP filter coefficients.
2. **Query Attempt (`0x0E02`) Rejected**:
   - Live probing with `0x0E02` (`REQUEST_RESPONSE_EQUALIZER_CONFIG`) across multiple payload variants (empty, band ID `0xA1`, `0x00`, `0x01`) consistently triggers the GAIA error frame:
     `FF 03 00 01 00 1D 0F 82 01` (`0x0F82` is `0x0E02 | 0x0180` error flag, payload `0x01` = `STATUS_NOT_SUPPORTED`).
   - The headset does not provide a mechanism to read back active PEQ coefficients from the DSP registers.
3. **Client-Side State Tracking & Persistence**:
   - Both the official EarFun companion app and Gadgetbridge maintain preset definitions and active preset states entirely within client-side local storage (`SharedPreferences` in Android).
   - In WaveTune, the selected preset is persisted in `~/.config/wavetune/settings.ini` under an anonymous device key derived from the SHA-256 hash of the device MAC (`[Device_{PID}_{SHA256(MAC)[..12]}]`). This guarantees uniqueness across multiple headsets of the same model without persisting or exposing raw Bluetooth addresses.
   - On session establishment, WaveTune checks if a non-default preset was previously active and automatically synchronizes the 10-band DSP parameters to the earbud, ensuring consistent audio reproduction even after headset power cycles.

#### 10-Band Models (EarFun Air Pro 4 & Free Pro 3):
| Band | Band ID | Frequency (Hz) | Q-Factor |
|---|---|---|---|
| 1 | `0xA1` | 31.5 Hz (`0x005E`) | `0x0B33` |
| 2 | `0xA2` | 63 Hz (`0x00BD`) | `0x0B33` |
| 3 | `0xA3` | 125 Hz (`0x0177`) | `0x0B33` |
| 4 | `0xA4` | 250 Hz (`0x02EE`) | `0x0B33` |
| 5 | `0xA5` | 500 Hz (`0x05DC`) | `0x0B33` |
| 6 | `0xA6` | 1000 Hz (`0x0BB8`) | `0x0B33` |
| 7 | `0xA7` | 2000 Hz (`0x1770`) | `0x0B33` |
| 8 | `0xA8` | 4000 Hz (`0x2EE0`) | `0x0B33` |
| 9 | `0xA9` | 8000 Hz (`0x5DC0`) | `0x0B33` |
| 10 | `0xAA` | 16000 Hz (`0xBB80`) | `0x0B33` |

#### 6-Band Models (EarFun Air S):
The EarFun Air S uses 6 active EQ frequency bands (60 Hz, 150 Hz, 400 Hz, 1000 Hz, 3000 Hz, 8000 Hz).
However, the GAIA protocol expects all 10 bands to be sent. Bands 7–10 are dummy bands populated with fixed default values (frequencies 31.5 Hz, 63 Hz, 125 Hz, 250 Hz, Q-Factor `0x0B33`, Gain 0 dB) as specified in Gadgetbridge's `Equalizer.java`.

---

### 4.7 Audio Codec Selection
- **Query**: `0x032F` $\to$ Response `0x832F [00, codec]`
- **Set**: `0x032E [codec]`
- **Codec Values**:
  - `0`: Stable Connection (SBC/AAC)
  - `1`: aptX
  - `3`: aptX Adaptive
  - `8`: LDAC
  - `19` (`0x13`): aptX Lossless

---

### 4.8 Voice Prompt Volume
- **Query**: `0x0339` $\to$ Response `0x8339 [00, vol]`
- **Set**: `0x0338 [vol]`
- **Scale**: `0` (Maximum) to `4` (Minimum / Mute).

---

### 4.9 Touch Mode and Gestures
- **Touch Lock Mode**:
  - Query: `0x0335` $\to$ Response `0x8335 [00, mode]` (`0` = Touch Enabled, `1` = Touch Disabled)
  - Set: `0x0334 [mode]`
- **Touch Gestures Setting**:
  - Set: `0x030A [interaction_type, action, 0x00, 0x00]` for Left, `[0x00, 0x00, interaction_type, action]` for Right.
  - Interaction Types:
    - Single Tap: `0x01`
    - Double Tap: `0x02`
    - Triple Tap: `0x03`
    - Long Press: `0x04`
  - Action Codes:
    - `16` (`0x10`): None
    - `1`: Play / Pause
    - `2`: Volume Up
    - `3`: Volume Down
    - `4`: Previous Track
    - `5`: Next Track
    - `6`: Voice Assistant
    - `7`: Game Mode Toggle
    - `8`: Noise Control Toggle (All Modes)

---

## 5. Live Verification Summary on EarFun Air Pro 4

Live tests performed on Linux against EarFun Air Pro 4 (Hardware-verified):

```text
RFCOMM Connection to Channel 11: SUCCESS

[TX] Firmware Query:    FF 04 00 00 00 0A 03 09
[RX] Firmware Response: FF 03 00 1B 00 0A 83 09 00 54 57 36 30 30 2D 33 30 39 31 5F 32 30 32 35 30 39 31 39 5F 76 30 2E 34 2E 31
     -> "TW600-3091_20250919_v0.4.1"

[TX] Battery Left:      FF 04 00 00 00 0A 03 06
[RX] Battery Left:      FF 03 00 02 00 0A 83 06 00 5A  (90%)

[TX] Battery Right:     FF 04 00 00 00 0A 03 07
[RX] Battery Right:     FF 03 00 02 00 0A 83 07 00 64  (100%)

[TX] Battery Case:      FF 04 00 00 00 0A 03 17
[RX] Battery Case:      FF 03 00 02 00 0A 83 17 00 64  (100%)

[TX] Ambient Mode:      FF 04 00 00 00 0A 03 15
[RX] Ambient Mode:      FF 03 00 02 00 0A 83 15 00 00  (Normal / Off)

[TX] ANC Sub-Mode:      FF 04 00 00 00 0A 03 3B
[RX] ANC Sub-Mode:      FF 03 00 02 00 0A 83 3B 00 04  (AI Environment Adaptive)

[TX] Codec Query:       FF 04 00 00 00 0A 03 2F
[RX] Codec Response:    FF 03 00 02 00 0A 83 2F 00 13  (aptX Lossless)

[TX] Set 10-Band EQ:    FF 04 00 09 00 1D 0E 01 A1 FF 98 00 5E 01 20 0B 33 ... (10 frames)
[RX] 10-Band EQ ACKs:   FF 03 00 01 00 1D 0F 81 01 (x10 ACKs received)

[TX] Query EQ (0x0E02): FF 04 00 00 00 1D 0E 02
[RX] Query Rejected:    FF 03 00 01 00 1D 0F 82 01 (Error 0x0F82, STATUS_NOT_SUPPORTED)
```

