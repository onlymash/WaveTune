// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use crate::protocol::{BatteryState, WearState};

use super::brands::oppo::{initialize_oppo_session, open_oppo_rfcomm};
use super::discovery::find_devices;
use super::snapshot::DeviceSnapshot;

pub fn probe() -> Result<(), String> {
    let mut devices = find_devices()?;
    if devices.len() != 1 {
        return Err(if devices.is_empty() {
            crate::t!("device-none-connected")
        } else {
            crate::t!("device-found-multiple")
        });
    }
    let device = devices.remove(0);
    match device.brand {
        crate::bluetooth::DeviceBrand::EarFun => super::brands::earfun::probe_earfun(&device),
        _ => {
            let mut stream = open_oppo_rfcomm(&device)
                .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
            let (snapshot, _) = initialize_oppo_session(&mut stream, &device.name, None)?;
            print_snapshot(&snapshot);
            Ok(())
        }
    }
}

pub(crate) fn print_snapshot(snapshot: &DeviceSnapshot) {
    println!(
        "{}",
        crate::t!("probe-device", snapshot.identity.name.as_str())
    );
    println!(
        "{}",
        crate::t!("probe-model-profile", snapshot.identity.model.as_str())
    );
    println!(
        "{}",
        crate::t!(
            "probe-product-id",
            format!("{:06X}", snapshot.identity.product_id)
        )
    );
    if let Some(color_id) = snapshot.identity.color_id {
        println!("{}", crate::t!("probe-color-id", color_id));
    }
    print_battery(&snapshot.battery);
    if snapshot.wear.supports_wear_detection {
        print_wear(&snapshot.wear.wear);
    } else {
        println!("{}", crate::t!("wear-status-not-supported"));
    }
    fn clean(s: &Option<String>) -> Option<&str> {
        s.as_deref().map(str::trim).filter(|v| !v.is_empty())
    }
    let left = clean(&snapshot.identity.firmware.left);
    let right = clean(&snapshot.identity.firmware.right);
    let case = clean(&snapshot.identity.firmware.case);

    match (left, right, case) {
        (Some(l), Some(r), Some(c)) => {
            println!("{}", crate::t!("probe-firmware", l, r, c));
        }
        (Some(l), Some(r), None) => {
            if l == r {
                println!("{}", crate::t!("probe-firmware-single", l));
            } else {
                println!("{}", crate::t!("probe-firmware-left-right", l, r));
            }
        }
        (Some(l), None, None) => {
            println!("{}", crate::t!("probe-firmware-single", l));
        }
        (None, Some(r), None) => {
            println!("{}", crate::t!("probe-firmware-single", r));
        }
        _ => {
            let not_reported = crate::t!("common-not-reported");
            println!(
                "{}",
                crate::t!(
                    "probe-firmware",
                    left.unwrap_or(&not_reported),
                    right.unwrap_or(&not_reported),
                    case.unwrap_or(&not_reported),
                )
            );
        }
    }
    if snapshot.identity.read_only {
        let unconfirmed = crate::t!("common-unconfirmed");
        println!(
            "{}",
            crate::t!(
                "probe-detected-read-only",
                if snapshot.diagnostics.detected_features.is_empty() {
                    unconfirmed
                } else {
                    snapshot.diagnostics.detected_features.join(", ")
                }
            )
        );
        for error in &snapshot.diagnostics.probe_errors {
            println!("{}", crate::t!("probe-note", error));
        }
        println!("{}", crate::t!("controls-not-enabled"));
        return;
    }
    if let Some(mode) = snapshot.noise.current_mode {
        println!("{}", crate::t!("probe-noise-mode-index", mode));
    } else {
        println!("{}", crate::t!("noise-mode-not-returned"));
    }
    println!(
        "{}",
        crate::t!(
            "probe-noise-mode-support",
            snapshot.noise.support_note.as_str()
        )
    );
    if snapshot.sound.supports_equalizer {
        if let Some(preset) = snapshot.sound.equalizer_preset {
            let presets =
                crate::compatibility::equalizer_presets_for_product(snapshot.identity.product_id);
            let preset_name = crate::compatibility::equalizer_preset_key(preset, presets)
                .map(|key| crate::t!(key))
                .unwrap_or_else(|| preset.to_string());
            println!("{}", crate::t!("probe-headset-eq", preset_name));
        } else {
            println!("{}", crate::t!("equalizer-not-returned"));
        }
    }
    if snapshot.sound.supports_custom_equalizer {
        if !snapshot.sound.custom_equalizers.is_empty() {
            let names: Vec<String> = snapshot
                .sound
                .custom_equalizers
                .iter()
                .map(|eq| {
                    if eq.is_selected {
                        format!("{} (ID {}, selected)", eq.name, eq.id)
                    } else {
                        format!("{} (ID {})", eq.name, eq.id)
                    }
                })
                .collect();
            println!("{}", crate::t!("probe-custom-eq-list", names.join(", ")));
        } else {
            println!("{}", crate::t!("probe-custom-eq-none"));
        }
    }
    if snapshot.sound.supports_spatial_audio {
        if let Some(mode) = snapshot.sound.spatial_audio_mode {
            let mode_key = crate::compatibility::spatial_audio_mode_key(mode);
            println!("{}", crate::t!("probe-spatial-audio", crate::t!(mode_key)));
        } else {
            println!("{}", crate::t!("spatial-audio-not-returned"));
        }
    }
    if snapshot.sound.supports_game_mode {
        if let Some(enabled) = snapshot.sound.game_mode_enabled {
            let state_str = if enabled {
                crate::t!("common-on")
            } else {
                crate::t!("common-off")
            };
            println!("{}", crate::t!("probe-game-mode", state_str));
        } else {
            println!("{}", crate::t!("game-mode-not-returned"));
        }
    }
    if snapshot.sound.supports_bass_engine {
        if let Some(enabled) = snapshot.sound.bass_engine_enabled {
            let state_str = if enabled {
                crate::t!("common-on")
            } else {
                crate::t!("common-off")
            };
            if let Some(level) = &snapshot.sound.bass_engine_level {
                println!(
                    "{}",
                    crate::t!(
                        "probe-bass-engine",
                        state_str,
                        level.current,
                        level.min,
                        level.max
                    )
                );
            } else {
                println!("{}", crate::t!("probe-bass-engine-simple", state_str));
            }
        } else {
            println!("{}", crate::t!("bass-engine-not-returned"));
        }
    }
    if snapshot.sound.supports_hi_res {
        if let Some(enabled) = snapshot.sound.hi_res_enabled {
            let state_str = if enabled {
                crate::t!("common-on")
            } else {
                crate::t!("common-off")
            };
            println!("{}", crate::t!("probe-hi-res", state_str));
        } else {
            println!("{}", crate::t!("hi-res-not-returned"));
        }
    }
    if snapshot.sound.supports_audio_codec {
        if let Some(codec) = snapshot.sound.audio_codec {
            let codec_name = match codec {
                0 => "SBC",
                1 => "AAC",
                2 => "aptX",
                19 => "LDAC",
                _ => "Unknown",
            };
            println!("{}", crate::t!("probe-audio-codec", codec_name));
        }
    }
    if snapshot.dual_device.supports_dual_device {
        if let Some(dual) = snapshot.dual_device.dual_device_enabled {
            let state_str = if dual {
                crate::t!("common-on")
            } else {
                crate::t!("common-off")
            };
            println!("{}", crate::t!("probe-dual-device", state_str));
        }
        match &snapshot.dual_device.paired_devices {
            Ok(devs) => {
                if devs.is_empty() {
                    println!("{}", crate::t!("dual-device-none-connected"));
                } else {
                    println!("{}", crate::t!("dual-device-list-title"));
                    for dev in devs {
                        let conn_str = if dev.connected {
                            format!(" ({})", crate::t!("dual-device-connected"))
                        } else {
                            String::new()
                        };
                        println!("  - {}{conn_str}", dev.name);
                    }
                }
            }
            Err(error) => {
                println!("{}", crate::t!("probe-dual-device-error", error.as_str()));
            }
        }
    }
}

fn print_battery(state: &BatteryState) {
    let is_headphone = (state.left.is_some() && state.right.is_none() && state.case.is_none())
        || (state.left.is_none() && state.right.is_some() && state.case.is_none());

    if is_headphone {
        if let Some(part) = state.left.or(state.right) {
            let charging_suffix = if part.charging {
                format!(" ({})", crate::t!("battery-charging"))
            } else {
                String::new()
            };
            println!(
                "{}: {}%{charging_suffix}",
                crate::t!("battery-headset"),
                part.percentage
            );
            return;
        }
    }

    for (name, part) in [
        (crate::t!("battery-left-ear"), state.left),
        (crate::t!("battery-right-ear"), state.right),
        (crate::t!("battery-case"), state.case),
    ] {
        match part {
            Some(part) => {
                let charging_suffix = if part.charging {
                    format!(" ({})", crate::t!("battery-charging"))
                } else {
                    String::new()
                };
                println!("{name}: {}%{charging_suffix}", part.percentage);
            }
            None => println!("{}", crate::t!("probe-not-reported", name)),
        }
    }
}

fn print_wear(state: &WearState) {
    for (name, part) in [
        (crate::t!("battery-left-ear"), state.left),
        (crate::t!("battery-right-ear"), state.right),
    ] {
        if let Some(part) = part {
            let out_status = if part.out_of_case {
                crate::t!("wear-removed")
            } else {
                crate::t!("wear-in-case")
            };
            let in_ear_status = if part.in_ear {
                crate::t!("wear-in-ear")
            } else {
                crate::t!("wear-out-of-ear")
            };
            println!("{name}: {out_status}, {in_ear_status}");
        }
    }
    if let Some(open) = state.case_open {
        let lid_status = if open {
            crate::t!("wear-case-open")
        } else {
            crate::t!("wear-case-closed")
        };
        println!("{}: {}", crate::t!("battery-case"), lid_status);
    }
}
