# Instructions for AI coding agents

## Project overview

WaveTune is an unofficial multi-brand wireless headset control center for Linux (ported from EncoDeck),
written in Rust with a GTK4 / Libadwaita UI. It is not affiliated with or endorsed by OPPO, OnePlus,
realme, EarFun, or any other vendor.

The project's goal is to bring comprehensive wireless headset control to Linux across multiple brands.
Currently, it provides full support for the OPPO family (OPPO, OnePlus, realme) and EarFun (Air Pro 4,
Free Pro 3, Air S, etc.). Brand-specific APIs, protocols, and configurations are segregated into dedicated
brand namespaces (`src/protocol/brands/` and `src/bluetooth/brands/`), with an extensible driver architecture
(`HeadsetDriver`, `AnyDriver`, and `SessionRunner`) prepared for future brand additions (such as Huawei,
Xiaomi, Vivo, Sony, and Edifier). Supported models are defined in the catalog and treated equally based
on their declared capabilities. Bluetooth control uses BlueZ for device discovery and Linux RFCOMM sockets
for private vendor control protocols.

Before changing behavior, read:

- [`README.md`](README.md) for build and runtime instructions.
- [`docs/OPPO_BLUETOOTH_API.md`](docs/OPPO_BLUETOOTH_API.md) for OPPO/OnePlus/realme protocol details.
- [`docs/EARFUN_BLUETOOTH_API.md`](docs/EARFUN_BLUETOOTH_API.md) for EarFun GAIA V3 protocol details.
- [`docs/ARCHITECTURE_GUIDELINES.md`](docs/ARCHITECTURE_GUIDELINES.md) for core MVI architecture and vertical slice patterns.
- [`docs/NEW_BRAND_INTEGRATION_GUIDE.md`](docs/NEW_BRAND_INTEGRATION_GUIDE.md) for new brand integration SOP.
- [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md) for scope and roadmap.

## Current implementation status

- GTK4 UI has Home, Sound, Controls, and Device pages, plus desktop system tray (StatusNotifierItem)
  and GNOME Quick Settings D-Bus integration.
- On startup, the UI discovers already-connected supported headsets (OnePlus, OPPO, realme, EarFun)
  using BlueZ headset category, Bluetooth company identifiers, manufacturer data, and brand control
  service UUIDs (such as Qualcomm GAIA UUID `00001107-d102-11e1-9b23-00025b00a5a5` for EarFun or OPPO
  control UUID fallback); aliases are display-only, not a filter. Do not use BlueZ `Modalias` as a
  brand identifier because it can identify the chipset vendor.
- On launch, the first discovered supported device is connected automatically (if none is found yet,
  later scans keep trying until one appears or the user picks a device manually). Discovery repeats every
  15 seconds and can also be triggered manually. While no control channel is connected, navigation pages
  show a shared empty state prompting the user to connect a headset. The app does not pair devices or
  establish the OS-level Bluetooth/audio connection.
- RFCOMM channel fallback is used when SDP discovery does not provide an explicit channel (channel 15
  for OPPO, channel 11 for EarFun).
- Multi-brand driver architecture: `create_driver(device.brand)` dispatches to brand drivers implementing
  `HeadsetDriver` (`OppoDriver`, `EarFunDriver`), executed uniformly by `SessionRunner`.
- Session initialization queries handshake/capabilities, product ID, battery, firmware versions,
  wear/case-lid state (if supported), noise-control mode (if supported), equalizer, and paired device
  management (if supported).
- Battery, wear, noise-mode, and paired-device events are parsed while connected. Battery displays adapt
  dynamically for full TWS (left/right/case), TWS without case, and over-ear headphones.
- `cargo run -- --probe` performs a one-shot read-only query across any supported brand and prints the snapshot.
- The GTK UI can open and close the selected device's control channel and displays observed device
  information. Switching devices closes the current control session before opening the next one.
- Noise controls: shows off, noise cancelling, transparency, and adaptive modes according to the model's
  profile and submodes. Writes require a success acknowledgement and matching readback.
- Noise controls remain disabled until the wear query/event reports at least one earbud in-ear (for
  headsets that support wear detection; if unsupported, this check is skipped); the driver checks wear
  state again before sending a setting.
- Sound controls: preset and custom equalizers (OPPO custom EQ, EarFun 6/10-band hardware EQ), spatial
  audio (with fixed/head-tracking modes where supported), game mode (low latency), bass engine, and audio
  codec selection (LDAC, aptX Adaptive, AAC, SBC) with conflict handling and reboot sequencing.
- Controls & Gestures: touch gestures customization, in-ear detection toggle, auto-sleep, prompt volume,
  head motion controls, fit test, and find earbuds.
- Device management: dual-device (multipoint) connection toggle, preferred device, and paired device
  connection/disconnection/removal management.
- Unknown devices (unrecognized product ID) remain strictly read-only to safeguard hardware.

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
- `src/app/state.rs`: MVI core state (`HeadsetSessionState`).
- `src/app/controller.rs`: coordinates device selection, connection lifecycle, and UI updates.
- `src/app/device_manager.rs`: starts discovery and RFCOMM monitor workers.
- `src/app/desktop/`: system tray (StatusNotifierItem) and GNOME Quick Settings D-Bus service.
- `src/app/features/`: vertical feature slices for UI components and controllers (`noise/`, `sound/`, `controls/`, `device/`).
- `src/ui/`: GTK window composition, home/sound/controls/device pages, shared components, and view-state updates. Keep protocol logic out of these modules.
- `src/bluetooth/`: BlueZ discovery and candidate filtering (`discovery.rs`), generic RFCOMM transport (`transport.rs`), unified `--probe` output (`probe.rs`), driver trait abstraction (`driver.rs`), common worker thread execution loop (`worker.rs`), domain snapshots (`snapshot.rs`), unified driver factory (`create_driver` in `mod.rs`), and modular brand drivers under `brands/` (`brands/oppo/`, `brands/earfun/`).
- `src/compatibility/`: multi-brand catalog architecture. Generic profile types and lookup in `mod.rs`, aggregate registry in `registry.rs`, and brand-specific catalogs under `brands/` (`brands/oppo/`, `brands/earfun/`).
- `src/protocol/`: generic models (`core/types.rs`) and brand-specific protocol codecs under `brands/` (`brands/oppo/`, `brands/earfun/`). Keep these modules independent of GTK and BlueZ.
- `docs/OPPO_BLUETOOTH_API.md`: recorded private-protocol facts and provenance for OPPO/OnePlus/realme.
- `docs/EARFUN_BLUETOOTH_API.md`: recorded private-protocol facts and GAIA V3 specs for EarFun.
- `docs/ARCHITECTURE_GUIDELINES.md`: core architecture guidelines, MVI pattern, and vertical slice design.
- `docs/NEW_BRAND_INTEGRATION_GUIDE.md`: standardized guide and code templates for integrating new headset brands.
- `docs/UI_COMPONENT_GUIDELINES.md`: architectural guidelines and specifications for GTK4/Libadwaita UI components.
- `packaging/`: Arch/AUR PKGBUILD generator and publisher (`arch/generate.sh` and `arch/publish.sh`), Debian control template (`debian/`), the shared file layout (`install.sh`), and `package.sh`, which builds packages into `target/dist/`.

Keep protocol parsing independent of GTK. Keep Bluetooth I/O off the GTK main thread and pass typed updates to the UI. Prefer extending existing typed state/event structures over embedding protocol byte handling in widgets.

## Bluetooth and protocol safety

- Treat all vendor protocols (OPPO family, EarFun GAIA V3, etc.) as unofficial and model-/firmware-specific. Consult the API notes before adding commands or changing parsing behavior.
- The API notes distinguish device-tested information from client-derived and community-derived facts. Preserve that evidence distinction in code comments, docs, and compatibility claims.
- Keep unknown product IDs strictly read-only.
- Never implement OTA or firmware flashing.
- Do not add generic protocol scanning, undocumented writes, or bulk commands.
- For write features, require a supported profile in `DEVICE_PROFILES` with the respective feature enabled; send one setting at a time; wait for acknowledgement and authoritative readback/event; retain the last confirmed UI state on error.
- Parse the RFCOMM stream incrementally using brand `FrameDecoder`s. A socket read is not guaranteed to contain exactly one protocol frame; enforce length and buffer limits.
- Surface BlueZ, socket, protocol, and device status errors. Do not turn errors into plausible default device values.

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
