# Instructions for AI coding agents

## Project overview

WaveTune is an unofficial multi-brand wireless headset control center for Linux (ported from EncoDeck),
written in Rust with a GTK4 / Libadwaita UI. It is not affiliated with or endorsed by OPPO, OnePlus,
realme, or any other vendor.

The project's goal is to bring EncoDeck's multi-brand wireless headset control to Linux. Currently,
it provides full support for the OPPO family (OPPO, OnePlus, realme). Brand-specific APIs, protocols,
and configurations are segregated into dedicated brand namespaces (`brands/oppo`), preparing the codebase
for future brand additions (such as Huawei, Xiaomi, Vivo, and Edifier). Supported models are defined in the
catalog and treated equally based on their declared capabilities. Bluetooth control uses BlueZ for device
discovery and a Linux RFCOMM socket for the private control protocol.

Before changing behavior, read:

- [`README.md`](README.md) for build and runtime instructions.
- [`docs/OPPO_BLUETOOTH_API.md`](docs/OPPO_BLUETOOTH_API.md) for protocol details and evidence.
- [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md) for scope and milestones.

## Current implementation status

- GTK4 UI has home, sound, controls, and device pages.
- On startup, the UI discovers already-connected OnePlus/OPPO/realme headsets using BlueZ headset
  category and Bluetooth company identifiers, with the headset control UUID as an OPPO fallback;
  aliases are display-only, not a filter. Do not use BlueZ `Modalias` as a brand identifier because
  it can identify the chipset vendor. On launch, the first discovered device with the control service
  is connected automatically (if none is found yet, later scans keep trying until one appears or the
  user picks a device manually); multiple devices are selectable, but only devices with that service
  can currently open the control channel. Discovery repeats every 15 seconds and can also be triggered
  manually. While no control channel is connected, all four navigation pages show a shared empty state
  prompting the user to connect a headset. The app does not pair devices or establish the OS-level
  Bluetooth/audio connection.
- RFCOMM channel 15 is used as the fallback when SDP discovery does not provide an explicit channel.
- Session initialization queries handshake/capabilities, product ID, available events, battery,
  firmware versions, wear/case-lid state (if supported), noise-control mode (if supported), and paired
  device management (if supported).
- Battery, wear, noise-mode, and paired-device events are parsed while connected.
- `cargo run -- --probe` performs a one-shot read-only query and prints the result.
- The GTK UI can open and close the selected device's control channel and displays observed device
  information. Switching devices closes the current control session before opening the next one.
- The GTK UI queries noise-mode support with `0x010C` subtype 3. If an all-zero bitmap or empty response
  is returned, the UI falls back to the model's catalog mode tree in `DEVICE_PROFILES`.
- The UI shows off, noise cancelling, transparency, and adaptive modes according to the model's profile.
  Writes require a success acknowledgement and matching readback.
- Noise controls remain disabled until the wear query/event reports at least one earbud in-ear (for
  headsets that support wear detection; if unsupported, this check is skipped); the worker checks wear
  state again before sending a setting.
- EQ write actions remain disabled.
- Device capabilities are determined uniformly by matching product ID against `DEVICE_PROFILES`.
  Unknown devices (unrecognized product ID) remain read-only.

The local device last reported left/right/case firmware `128/128/100`, 60% for both earbuds, both
earbuds removed from the case and in-ear, case lid open, and noise-mode index 13. These are
time-specific observations, not defaults or constants; never hard-code them into the UI.

## Development environment

The project was developed and validated with:

- Arch Linux, GNOME / Wayland
- GTK `4.24.1` from the Arch `gtk4` package
- Rust stable (the recorded environment was Rust/Cargo `1.99.1`)
- Rust GTK bindings `gtk4 0.11.5`
- BlueZ `5.87`

Use the versions available on the current machine; do not assume another workstation has these
exact versions. Check with:

```bash
pacman -Qi gtk4 bluez
pkg-config --modversion gtk4
rustup show active-toolchain
rustc --version
cargo --version
systemctl is-active bluetooth
```

Install dependencies on Arch Linux when needed:

```bash
sudo pacman -S --needed bluez gtk4 pkgconf rustup
rustup toolchain install stable
rustup default stable
sudo systemctl enable --now bluetooth
```

The headset must first be paired and connected through GNOME Bluetooth settings. The app does not
pair devices or establish the OS-level headset/audio connection.

## Build, run, and test

From the repository root:

```bash
cargo fmt --check
cargo test --locked
cargo check --locked
cargo run
```

To query a connected, supported headset once:

```bash
cargo run --locked -- --probe
```

Do not run Bluetooth probes concurrently with another app session using the headset's RFCOMM
control service; the device may accept only one control connection at a time.

## Code organization

- `src/main.rs`: GTK application startup and `--probe` dispatch.
- `src/app/events.rs`: typed commands and events exchanged between GTK and background work.
- `src/app/controller.rs`: coordinates device selection, connection lifecycle, and UI updates.
- `src/app/device_manager.rs`: starts discovery and RFCOMM monitor workers.
- `src/ui/`: GTK window composition, home/sound/controls/device pages, shared components, and view-state
  updates. Keep protocol logic out of these modules.
- `src/bluetooth/`: BlueZ discovery and candidate filtering (`discovery.rs`), generic RFCOMM transport
  (`transport.rs`), read-only `--probe` output (`probe.rs`), and brand-specific control sessions
  under `brands/` (OPPO session implementation in `brands/oppo/oppo_session.rs`).
- `src/compatibility/`: multi-brand catalog architecture. Generic profile types and lookup in `mod.rs`,
  aggregate registry in `registry.rs`, and brand-specific catalogs under `brands/`
  (OPPO/OnePlus/realme models in `brands/oppo/oppo_catalog.rs`).
- `src/protocol/`: generic models (`core/types.rs`) and brand-specific protocol codecs under `brands/`
  (OPPO frames `oppo_frame.rs`, packets `oppo_packet.rs`, commands `oppo_commands.rs`, capabilities
  `oppo_capabilities.rs`, parsers `oppo_parsers.rs`, and noise-mode tree `oppo_noise.rs`). Keep these
  modules independent of GTK and BlueZ.
- `docs/OPPO_BLUETOOTH_API.md`: recorded private-protocol facts and provenance.
- `docs/IMPLEMENTATION_PLAN.md`: product scope, implementation phases, and acceptance criteria.
- `packaging/`: Arch/AUR PKGBUILD generator and publisher (`arch/generate.sh` and `arch/publish.sh`,
  which generate and publish standalone git-ignored AUR repositories in `arch/wavetune` and
  `arch/wavetune-git` from `Cargo.toml` and git metadata), Debian control template (`debian/`), the
  shared file layout (`install.sh`), and `package.sh`, which builds packages into `target/dist/`.

Keep protocol parsing independent of GTK. Keep Bluetooth I/O off the GTK main thread and pass typed
updates to the UI. Prefer extending existing typed state/event structures over embedding protocol
byte handling in widgets.

## Bluetooth and protocol safety

- Treat the OPPO protocol as unofficial and model-/firmware-specific. Consult the API notes before
  adding commands or changing parsing behavior.
- The API notes distinguish device-tested information from client-derived and community-derived
  facts. Preserve that evidence distinction in code comments, docs, and compatibility claims.
- Keep unknown product IDs read-only.
- Never implement OTA or firmware flashing.
- Do not add generic protocol scanning, undocumented writes, or bulk commands.
- For write features, require a supported profile in `DEVICE_PROFILES` with the respective feature enabled;
  send one setting at a time; wait for acknowledgement and authoritative readback/event; retain the
  last confirmed UI state on error.
- Parse the RFCOMM stream incrementally. A socket read is not guaranteed to contain exactly one
  protocol frame; enforce length and buffer limits.
- Surface BlueZ, socket, protocol, and device status errors. Do not turn errors into plausible
  default device values.

## Privacy

Bluetooth device addresses, serial numbers, and device-list events can expose personal information
or details about other paired devices.

- Do not log or persist raw Bluetooth addresses, raw protocol frames, serial numbers, or other
  paired-device names/addresses.
- Query `0x0112` only when supported by the device profile for paired-device management; discard addresses
  while parsing, keep names in live UI/session memory only, and never log, export, or persist them.
- Do not query the sensitive serial command `0x0117`.
- If diagnostics are added, redact identifiers before display or export and make export an
  explicit user action.
- Never add real device identifiers or locally observed battery/status values as application
  defaults or fixtures unless they are anonymized test data.

## Logging and internationalization

- Console log messages, probe output, and diagnostic prints via `println!`, `eprintln!`, or `tracing` must use plain English and do NOT need internationalization (`crate::t!`).
- Only user-facing text displayed in the GTK/Libadwaita UI (dialogs, labels, settings, status notifications, etc.) requires internationalization through Fluent translation files (`locales/`).

## Documentation and change expectations

- Update `docs/OPPO_BLUETOOTH_API.md` when protocol facts or confidence levels change.
- Update `docs/IMPLEMENTATION_PLAN.md` and `README.md` when current capabilities, setup, or scope
  change.
- Do not claim hardware support based only on successful compilation or mock tests. State the exact
  model, firmware, and test evidence when reporting hardware validation.
- Keep dependencies at current stable releases compatible with the repository's locked build, and
  commit `Cargo.lock` changes when dependency manifests change.
- Keep generated build output under `target/`; it is ignored by Git.
