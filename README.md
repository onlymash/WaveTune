# WaveTune

**English** | [简体中文](README_zh.md)

An unofficial multi-brand wireless headset control center for the Linux desktop (ported from [EncoDeck](https://github.com/TIANLI0/EncoDeck)), built with Rust and GTK4 / Libadwaita. It currently supports OPPO, OnePlus, and realme wireless headsets, with a decoupled multi-brand architecture ready for expanding to additional brands in the future.

> This project is not affiliated with or endorsed by any headset vendor. See [`docs/OPPO_BLUETOOTH_API.md`](docs/OPPO_BLUETOOTH_API.md) for private Bluetooth protocol details and hardware verification notes.

## Features

- **Status Monitoring & 3D Preview**: Real-time battery levels and charging status for left/right earbuds and charging case, in-ear wear and case-lid detection, firmware version display, device artwork, and interactive 3D model preview.
- **Noise Control**: Switch between Active Noise Cancellation (with multi-level intensity), Transparency mode (including Vocal Enhancement), and Adaptive noise control.
- **Sound & Audio Effects**: Built-in sound presets, custom multi-band Equalizer (EQ), Spatial Audio (Fixed / Head Tracking), and low-latency Game Mode.
- **Controls & Utilities**: Customize left/right touch and call gestures, long-press noise-control cycling, dual-device connection and paired-host switching, in-ear auto-pause, sleep detection, earbud fit test, Find My Earbuds, and prompt volume adjustment (available features depend on headset model).
- **Desktop Integration**: Native GTK4 / Libadwaita UI with English/Chinese localization, selectable app icon styles, background mode and silent login autostart, plus a companion GNOME Shell Quick Settings extension (GNOME 45–51).

## Installation

### Arch Linux (AUR)

Users on Arch Linux and its derivatives can install [`wavetune-git`](https://aur.archlinux.org/packages/wavetune-git) from the AUR:

```bash
paru -S wavetune-git
# or with yay
yay -S wavetune-git
```

Or clone and build manually:

```bash
git clone https://aur.archlinux.org/wavetune-git.git
cd wavetune-git
makepkg -si
```

Before launching WaveTune, make sure the system `bluetooth` service is running and pair/connect your headset via your desktop's Bluetooth settings first (WaveTune attaches to already-connected headsets rather than handling OS-level pairing):

```bash
sudo systemctl enable --now bluetooth
```

## GNOME Quick Settings Extension

A companion extension for GNOME 45–51 is included at `gnome-shell/wavetune@onlymash.github.com` (automatically installed to the system extensions directory by distribution packages).

For local installation from source:

```bash
./gnome-shell/install-local.sh
gnome-extensions enable wavetune@onlymash.github.com
```

> In a Wayland session, if `gnome-extensions` does not recognize the UUID on first install, log out and back into your desktop session once before running the enable command.

When a supported headset is connected, a headset tile appears in the GNOME Quick Settings panel for toggling noise cancellation, checking live battery levels, switching noise-control sub-modes, or opening the main window. "Keep Running in Background" is enabled by default (closing the window hides it to the background; choose "Quit" from the main menu or press `Ctrl+Q` to exit completely), and "Start in Background on Login" can be toggled in Preferences.

## Building from Source

Requires `bluez`, `gtk4`, `libadwaita` (`>= 1.9`), `pkgconf`, and the Rust `stable` toolchain. On Arch Linux:

```bash
sudo pacman -S --needed bluez gtk4 libadwaita pkgconf rustup
rustup default stable
```

Build and run:

```bash
cargo build --release   # Release binary at target/release/wavetune
cargo test
cargo run
```

CLI options:

- `--background` / `-b`: Start silently in the background without opening the main window.
- `--probe`: Run a one-shot read-only diagnostic query against a connected headset and print a summary.

## Packaging

Packaging scripts and templates live under `packaging/`:

```bash
packaging/package.sh            # Build Arch package if makepkg is available, otherwise deb
packaging/package.sh arch       # Build Arch package from working tree snapshot
packaging/package.sh git        # Build wavetune-git from local committed HEAD
packaging/package.sh deb        # Build Debian/Ubuntu .deb package (requires dpkg-deb and libadwaita >= 1.9)
```

Built packages are written to `target/dist/`. For AUR repository generation and publishing workflows, see [`packaging/arch/README.md`](packaging/arch/README.md).

## Compatibility

The built-in compatibility catalog covers 130+ OPPO, OnePlus, and realme headset models. Supported controls are enabled dynamically based on each headset's reported product ID, capability flags, and model profile; unrecognized models connect in a safe read-only mode limited to handshake and product identification.

## License

GPL-3.0-only
