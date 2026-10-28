# OPPO Enco Bluetooth API Notes

> This is an interoperability note, not an OPPO-published API specification. It summarizes the
> protocol findings documented by the reference project [TIANLI0/EncoDeck](https://github.com/TIANLI0/EncoDeck),
> pinned here to commit [`f10889d`](https://github.com/TIANLI0/EncoDeck/commit/f10889d626e4abc279dc9291e02fb766990c3911).
> Most concrete packet values below were verified on an OPPO Enco X4 (product ID `06EC10`,
> firmware L/R/case `116/116/100`) on 2026-10-20. The Linux client additionally verified a
> read-only handshake, battery/noise-mode, firmware, and wear-state queries against a paired Enco
> X4 on Arch Linux on 2026-10-22. That device reported firmware `128/128/100`. These findings must
> not be assumed to apply identically to every OPPO, OnePlus, or realme headset.

## Evidence labels and scope

- **Device-tested**: sent to or received from the reference project's Enco X4 and checked on-device.
- **Official-client-derived**: documented by that project from local analysis of HeyMelody client
  behavior/configuration, for interoperability only.
- **Community-derived**: reported by cited community projects.
- **Linux implementation note**: an adaptation proposal for this client; it is not part of the
  headset protocol.

The reference project reports that its protocol implementation is independent and records protocol
facts rather than copying other projects' implementation. See its
[protocol findings](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/PROTOCOL_FINDINGS.md),
[compatibility table](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/COMPATIBILITY.md),
and [research notes](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/RESEARCH.md).

## 1. Bluetooth transport

The control protocol uses a Bluetooth Classic RFCOMM serial stream (SPP-like), not the normal
audio stream.

| Item | Known value | Evidence |
|---|---|---|
| Enco X4 control service UUID | `0000079A-D102-11E1-9B23-00025B00A5A5` | Device-tested |
| Enco X4 RFCOMM channel | `15` | Device-tested |
| Enco X4 service name | `ELNK` | Device-tested |
| Common fallback service UUID | `00001107-D102-11E1-9B23-00025B00A5A5` | Official-client-derived |
| X4 speech/translation service | `0000079F-D102-11E1-9B24-00025B00A5A5`, channel `14` | Device-tested; not the control channel |
| LE Audio/GATT alternative | Service `0000079A-…`, write characteristic `0100079A-…`, notify characteristic `0200079A-…` | Official-client-derived; not implemented by reference |

The official client selects a UUID from model configuration and falls back to `00001107-…` when
one is not configured. Do not select the first SDP service that happens to match: resolve the
configured control service for the connected model. An Enco X4 SDP record does not contain
`00001107-…`.

The reference app expects the headset to have already been paired and connected by the operating
system (ACL link); it does not page/connect a disconnected headset itself. On Linux, a suitable
implementation is to use BlueZ D-Bus for paired-device discovery and connection state, then a
Linux RFCOMM socket for the control channel. SDP lookup is needed to resolve a channel unless an
explicit, device-tested channel is configured. These are Linux integration decisions, not
vendor-defined API requirements.

## 2. Link framing

One logical packet is carried in this frame:

```text
AA | LEN (little-endian base-128 varint) | CTRL | 00 | [FSN] | application packet
```

| Field | Meaning |
|---|---|
| `AA` | Start marker |
| `LEN` | Number of bytes after the LEN field, encoded as a 7-bit continuation varint |
| `CTRL` | Low two bits: `0` single frame, `1` first fragment, `2` middle fragment, `3` final fragment |
| `00` | Reserved byte, sent as zero |
| `FSN` | Fragment sequence number, present only when `CTRL != 0` |
| Application packet | Command packet described below |

For the X4, the control packet maximum is reported as 660 bytes over SPP (500 bytes for GATT).
The reference decoder handles partial reads, coalesced frames, and fragment reassembly. There is
no checksum, CRC, or encryption at this protocol layer. The Linux client should not assume one
socket read equals one complete frame.

## 3. Application packet

```text
CMD (u16 little-endian) | SEQ (u8) | PLEN (u16 little-endian) | payload (PLEN bytes)
```

- Command high byte identifies the group: `01` query, `02` event/notification, `03` OTA,
  `04` setting, `05` headset-originated request, `0F` debug (official-client-derived).
- A response command is `request_cmd | 0x8000`; the response echoes the request sequence number.
- The first response-payload byte is a status; `00` means success.
- Headset-pushed events use a headset-owned incrementing sequence number.
- Official client timeout: 5 seconds, without retry. Reference client timeout: 3 seconds.
- Some community implementations use fixed sequence values (`F0` for battery-related requests,
  `00` for feature-state queries); both were observed to receive X4 responses.

Device-tested battery-query example:

```text
Request:  AA 07 00 00 06 01 F0 00 00
Response: AA 0F 00 00 06 81 F0 08 00 00 03 01 C6 02 C6 03 46
```

## 4. Session initialization

The sequence verified by the reference project on Enco X4:

1. `0x0100` handshake; response `0x8100 [status][capability bitmap…]`.
2. `0x0103` product ID; X4 reports `06 EC 10` in the payload (product ID `06EC10`).
3. `0x010B` color ID; reports 1-byte color variant index (always allowed without bitmap check).
4. `0x0200` available-event query; X4 reports event IDs `01 02 03 04 08 0B F1 F2 F3`.
5. `0x0205 [count][event IDs…]` subscription. The reference subscribes only to events it handles
   (`01`, `02`, `03`); some events require a host reply.
6. Query firmware, battery, wear state, noise-control state, EQ, and supported feature states.

Respect the handshake capability bitmap and avoid commands it does not permit. The reference
records no authentication, key exchange, or heartbeat. On connection X4 may send requests such
as `0x0500` (time) and `0x051C` (app capabilities); the reference reports that ignoring these did
not prevent normal use on its tested X4.

For the Enco X4 capability bitmap, bit 8 is documented for noise-control query `0x010C` and
setting `0x0404`. The Linux client checks this bit before exposing or sending noise-mode settings.
Other command-to-capability mappings are not inferred from this single confirmed bit.

## 5. Common queries, events, and controls

All command IDs below are hexadecimal. Payload descriptions exclude the application status byte
where the response includes one.

| Feature | Query / event | Setting | Payload facts and evidence |
|---|---|---|---|
| Color ID | Query `0x010B` | — | Response `[status][color_id]`. Device-tested in EncoDeck on X4; always allowed without capability bit gating. |
| Battery | Query `0x0106`; event `0x0204`, event ID `01` | — | `[status][count]{component,value}`; component `1` left, `2` right, `3` case; value bit 7 is charging and bits 0–6 are percentage. Device-tested. |
| Wear / position | Query `0x0109`; event `0x0204`, event ID `02` | — | `[status][count]{component,flags}`; bit 0 out of case, bit 1 in ear, bit 2 zero means case lid closed. Arch Enco X4 query returned `00 03 01 07 02 07 03 04` (both buds out and in ear, lid open). Device-tested; bit definitions partly official-client-derived. |
| Noise control | Query `0x010C` | Set `0x0404` | Query payload starts with subtype. Subtype 1 current mode; subtype 2 long-press cycle; subtype 3 supported modes; subtype 4 adaptive-noise level. Set payload begins `01 01` then a little-endian bit mask for the protocol index. State changes also arrive in event ID `03`. Device-tested on X4. |
| EQ preset | Query `0x010F`; push `0x0504` | Set `0x0406` | Current preset query and push are device-tested; write command is official-client-derived and was not write-tested on X4. |
| Feature switches | Query `0x010D`; push `0x0503` | Set `0x0403` | Query `[count][id,value…]`; setting `[id,0|1]`. X4 states and read-after-write approach documented by reference. |
| Firmware | Query `0x0105` | — | Response `[status][count][UTF-8 CSV component,type,version triples]`. X4 has reported left/right/case `116/116/100` in the initial protocol test and `128/128/100` on the Arch Linux device. Device-tested. |
| Custom EQ | Query `0x0122`; push `0x0506` | Set `0x0418` | Read format `0x0122` includes preset metadata, UTF-8 name, and frequency/signed-gain pairs (`0x0506` push without status). Writes use `0x0418` (`01` add, `02` select/modify, `03` delete) followed by `0x0122` readback. Device-tested on X4. |
| Spatial audio | Query `0x012A` | Set `0x0422` | Modes `0/1/2` (off/fixed/head tracking); write behavior official-client-derived. |
| Game sound | Query `0x012B` | Set `0x0423` | X4 observed mode `3` and supported values `[3,0]`; write behavior official-client-derived. |
| Prompt volume | Query `0x0130` | Set `0x0427` | X4 value `6`, documented range `1–10`; write behavior official-client-derived. |
| BassWave | Query `0x0124`; event `0x0508`; switch `29` via `0x010D` | Set `0x041B`; switch `29` via `0x0403` | X4 range `-5…5`, current `0`; level push `0x0508 [min, max, cur]`; write behavior official-client-derived. |
| Hi-Res Audio | Query `0x010D` (feature `24`) | Set `0x0403` (feature `24`) | High quality audio mode switch; device-tested on Enco X4 with official app: can be enabled simultaneously with spatial audio (no mutex on X4). |
| Touch mapping | Query `0x0108` | Set `0x0401` | Read device-tested; write details official-client-derived. |

### Noise-control mode indices observed on Enco X4

| Index | Meaning | Evidence |
|---:|---|---|
| 0 | Top-level off | Write observed; X4 reports child index `3` when read back |
| 1 | Top-level noise cancelling | Write observed; readback reports the active child |
| 2 | Top-level transparency | Write observed; readback reports child index `8` |
| 3 | Off child | Device-tested |
| 4 | Deep noise cancelling | Device-tested |
| 5 | Moderate noise cancelling | Reported by X4; write not tested |
| 6 | Light noise cancelling | Device-tested |
| 7 | Smart noise cancelling | Reported by X4 |
| 8 | Transparency child | Reported by X4 |
| 11 | Top-level adaptive | Model configuration-derived |
| 12–14 | Adaptive children (less / standard / more noise) | `13` observed; adjacent meanings from model configuration |

For index 13, the mask requires two little-endian bytes (`00 20`). The headset reports child indices
even when a top-level mode was selected. Treat mode mappings as model-specific, not universal.

**Additional hardware evidence (user-reported, 2026-10-22):** The Enco X4 owner reports that they
tested changing every listed option on their headset and each took effect, including adaptive,
moderate noise cancelling, smart switching, and the adaptive range options. This complements the
reference project's per-index evidence above. The Linux client itself was not used to independently
run this full hardware matrix in that test; its automated write tests use a mock RFCOMM peer to check
payload encoding, ACK handling, and readback logic.

The supported-mode query is `0x010C [03 01]`. EncoDeck reports that the Enco X4 responds with an
all-zero bitmap, so this query is not a usable list of supported options on the tested X4 firmware.
The handshake capability bit 8 only gates the noise-control command family; it does not enumerate
individual modes. EncoDeck derives the X4 mode tree from the official HeyMelody model configuration:
top-level off/noise-cancelling/transparency/adaptive, with the documented child modes above. The
Linux client imports mode trees by product ID from the EncoDeck catalog. It queries subtype 3 first
and filters the matched model's tree when the headset returns a non-empty bitmap; an empty or
unparseable response falls back to that model configuration. This fallback is model-configuration
evidence, not a device-reported capability list.

### Errors and write discipline

The reference reports status `0x0E` as requiring the earbuds to be worn; `0x0F`/`0x09` as busy or
occupied by another device; and `0x10` as low battery. A setting should be considered successful
only after a success acknowledgement and an authoritative readback/event. If it fails, preserve the
last confirmed UI state and show an error rather than displaying an assumed state.

The Linux client uses the X4 wear query `0x0109` and wear event ID `02` to gate noise-mode controls.
It enables mode buttons only while at least one earbud is reported in-ear; unknown or not-worn state
keeps controls disabled. The worker repeats this check before sending a setting command.

Use the negotiated capability bitmap, a known model/product ID, and a documented model mapping
before offering writes. Unknown product IDs should remain read-only. Firmware OTA command group
`0x03xx` is explicitly out of scope for the reference project and should not be implemented.

The Linux UI builds its current noise controls from the matched profile. A profile entry is not
hardware verification: controls are gated by the known model mapping, capability bit 8, the runtime
support list (with the model fallback described above), and wear state. Every attempt still requires
a success ACK and a matching subtype-1 readback before the UI reports success. The X4 hardware
evidence above is specific to that device; other catalog entries have not been individually tested
by this Linux client.

## 5.9 Linux compatibility catalog

`src/compatibility/brands/oppo/oppo_catalog.rs` contains 131 unique product IDs imported from the generated EncoDeck
catalog at commit [`96727ef`](https://github.com/TIANLI0/EncoDeck/commit/96727ef881f4496f4868ce9cbc999f78f8b5e132):
85 entries from `OppoCatalog.g.cs` and 46 from `OppoCatalog.Supplement.g.cs`. The source data versions
are `117006008` and `16007000` respectively. The supplement identifies its model table as derived
from [OppoPodsManager](https://github.com/Zhaoyi-ya/OppoPodsManager), GPL-3.0-or-later, pinned here to
commit [`96eddba`](https://github.com/Zhaoyi-ya/OppoPodsManager/commit/96eddba02a666907e8583613cfc3461206808fba).
The imported facts are product IDs, model names, noise-mode type/index/parent relationships,
device-reported-only markers, and device capability flags (dual-device connection, wear detection,
fit detection, equalizer presets, custom equalizer, game mode, spatial audio, head tracking,
bass engine, find device, touch customization, prompt volume, sleep detection, swift pair,
voice command, head motion, and Hi-Res audio). No EncoDeck implementation code was copied.
The catalog currently has 43 profiles with noise trees, 51 profiles declaring dual-device management,
and 49 profiles declaring wear detection.

The current Linux implementation uses this catalog for noise-mode selection, paired-device
management, and wear-detection gating (bypassing wear checks for headsets without in-ear sensors).
Other capability flags describe model capabilities for future features or diagnostic information. Features
are enabled directly according to the compatibility catalog (`DEVICE_PROFILES`). Unknown product IDs remain limited to
handshake and product identification. X3 product ID `067410` has catalog-defined noise, wear-detection, and
dual-device options.

**Enco X3 hardware observation (2026-10-22):** On the paired Linux device, BlueZ reported the
`0000079A-D102-11E1-9B23-00025B00A5A5` service UUID. Its SDP `ELNK` record resolved to RFCOMM
channel 15. The headset completed the `0x0100` handshake and returned product ID `067410` for
`0x0103`. This verifies transport and identification only. No other X3 query or setting has been
validated. The Linux client now attempts read-only event-list/subscription, battery (`0x0106`),
firmware (`0x0105`), and wear-state (`0x0109`) queries for the identified X3; each result is treated
as unverified until confirmed on X3 hardware, and settings remain disabled. These exploratory
requests are not evidence that X3 supports the corresponding features. Unknown product IDs still
receive only the handshake and product-ID query.

For other detected OnePlus/OPPO/realme headsets, the client attempts to find the named `ELNK` SPP service
through SDP and uses the channel from that service record. It does not assume channel 15 or use the
first arbitrary SPP record. After connection, only the handshake and product-ID query are sent;
unrecognized products remain read-only.

## 5.10 Earbud-paired device list

Query `0x0112` and device-list event `0x0204` / event ID `06` report devices paired to the headset,
not devices paired with the Linux computer. The documented record format is
`[count]{address[6] LE, entry_length, state, flags, name_length, UTF-8 name, optional data…}`.
`state != 0` indicates connected; flag bit 0 marks the current host, bit 1 the main audio device,
bit 2 active playback, and bits 3–5 the device type. This format and state interpretation are based
on EncoDeck's X4 protocol findings; the Linux implementation has parser tests and has read live X4
device-list responses.

The GTK device-management card displays the reported devices as a vertical list with a device-type
emoji, name/current-host marker, and a connect/disconnect/remove button in each row. Controls are exposed
for catalog profiles declaring dual-device management (`supports_dual_device`), and check dual-device
connection feature ID `17` via `0x010D`; the current host cannot be disconnected or removed from this list.
The documented `0x0429` actions are `01` connect, `02` disconnect, and `03` remove (with `0x0112` query on bit 29
and `0x0429` action on bit 59 in protocol bit definitions). On 2026-10-22, this
Linux client disconnected one connected non-current host and reconnected it on an OPPO Enco X4; both
operations returned success ACKs and matching `0x0112` readbacks.
Each operation requires a success ACK followed by a `0x0112` readback matching the requested state.
Before enabling the buttons, the client queries `0x010D` for feature ID `17` (dual-device
connection) and requires the returned value to be nonzero. The feature query uses
`[count][feature IDs…]` and returns `[status][count]{feature ID, value}`; this layout and feature
meaning are official-client-derived.

The parser retains device addresses only in live session memory so a user-initiated action can name
its target. It reverses the list's little-endian address bytes into the command's display order.
Addresses and names are not logged, exported, or persisted, and are cleared when the control session
ends. A live X4 response reported type value `2` for the current host (`flags=0x17`) and type value
`1` for other entries; the UI maps these to computer and phone emoji respectively. Other type values
remain provisional presentation choices; unknown values use a generic Bluetooth emoji.

## 5.11 Sound Quality and Effects (Equalizer, Spatial Audio, Game Mode)

Based on EncoDeck protocol findings, the private protocol supports reading and writing sound settings:

- **Equalizer Presets (EQ)**:
  - Query: `0x010F []` -> response `0x810F [status, preset_index]`.
  - Write: `0x0406 [preset_index]` -> ACK `0x8406 [0]`.
  - Unsolicited event: `0x0504 [preset_index]`.
  - Preset mapping: Models with Dynaudio co-tuning (e.g. Enco X, X2, X3, X4) use index mapping `0` 淳厚人声, `7` 丹拿特调, `2` 悠扬古典, `1` 纯粹原音, `3` 动感重低音; other models use `0` 明亮人声, `1` 纯正原音, `2` 纯净人声, `3` 动感重低音.
- **Spatial Audio**:
  - Query: `0x012A []` -> response `0x812A [status, mode]`.
  - Write: `0x0422 [mode]` -> ACK `0x8422 [0]`.
  - Modes: `0` 关闭, `1` 固定, `2` 头部追踪 (requires `supports_head_tracking`).
- **Game Mode (Low Latency)**:
  - Query: `0x010D [1, feature_id]` -> response `0x810D [status, count, feature_id, value]`.
  - Write: `0x0403 [feature_id, value]` (`value`: `1` for enabled, `0` for disabled) -> ACK `0x8403 [0]`.
  - Unsolicited event: `0x0503 [feature_id, value]`.
  - Feature ID: `0x28` (40) for Enco X4; `0x06` (6) for other models without game sound list.

Writes follow the safe pipeline: verify model capability -> write command -> wait for ACK -> verify matching readback -> retain previous confirmed UI state on error.

## 6. Sensitive data and privacy

The device-list event (`0x0204`, event ID `06`) and query `0x0112` can include Bluetooth addresses
and names of other paired devices. Query them only for catalog profiles declaring dual-device
management, retain addresses only in live session memory for explicit connect/disconnect actions,
and do not log, persist, or export addresses or names. The reference also marks
serial/model-identification command `0x0117` as sensitive and does not use it.

## 7. Sources and limitations

- [EncoDeck protocol findings](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/PROTOCOL_FINDINGS.md)
- [EncoDeck compatibility](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/COMPATIBILITY.md)
- [EncoDeck research notes](https://github.com/TIANLI0/EncoDeck/blob/f10889d626e4abc279dc9291e02fb766990c3911/docs/RESEARCH.md)
- [EncoDeck generated model catalog](https://github.com/TIANLI0/EncoDeck/blob/96727ef881f4496f4868ce9cbc999f78f8b5e132/src/EncoDeck.Protocol/Oppo/Models/OppoCatalog.g.cs) and [supplement](https://github.com/TIANLI0/EncoDeck/blob/96727ef881f4496f4868ce9cbc999f78f8b5e132/src/EncoDeck.Protocol/Oppo/Models/OppoCatalog.Supplement.g.cs)
- [Bluetooth SIG company identifiers](https://bitbucket.org/bluetooth-SIG/public/src/main/assigned_numbers/company_identifiers/company_identifiers.yaml), including realme `0x08A4`.
- [OppoPodsWindows](https://github.com/3295074384/OppoPodsWindows), [OppoPodsManager](https://github.com/Zhaoyi-ya/OppoPodsManager), and [OppoPods](https://github.com/Leaf-lsgtky/OppoPods) are named in the reference as community research sources.

This is an evolving, unofficial record, not a complete specification. Device-tested claims above
are specific to the reported Enco X4 firmware/test setup unless stated otherwise. Validate every
write on hardware before declaring support for a model or firmware.
