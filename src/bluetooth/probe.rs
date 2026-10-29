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

use super::brands::oppo::{OppoDeviceSnapshot, initialize_oppo_session, open_oppo_rfcomm};
use super::discovery::find_devices;

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
    let mut stream = open_oppo_rfcomm(&device)
        .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
    let (snapshot, _) = initialize_oppo_session(&mut stream, &device.name, None)?;
    print_snapshot(&snapshot);
    Ok(())
}

fn print_snapshot(snapshot: &OppoDeviceSnapshot) {
    println!("{}", crate::t!("probe-device", snapshot.name.as_str()));
    println!(
        "{}",
        crate::t!("probe-model-profile", snapshot.model.as_str())
    );
    println!(
        "{}",
        crate::t!("probe-product-id", format!("{:06X}", snapshot.product_id))
    );
    if let Some(color_id) = snapshot.color_id {
        println!("{}", crate::t!("probe-color-id", color_id));
    }
    print_battery(&snapshot.battery);
    if snapshot.supports_wear_detection {
        print_wear(&snapshot.wear);
    } else {
        println!("{}", crate::t!("wear-status-not-supported"));
    }
    let not_reported = crate::t!("common-not-reported");
    println!(
        "{}",
        crate::t!(
            "probe-firmware",
            snapshot.firmware.left.as_deref().unwrap_or(&not_reported),
            snapshot.firmware.right.as_deref().unwrap_or(&not_reported),
            snapshot.firmware.case.as_deref().unwrap_or(&not_reported),
        )
    );
    if snapshot.read_only {
        let unconfirmed = crate::t!("common-unconfirmed");
        println!(
            "{}",
            crate::t!(
                "probe-detected-read-only",
                if snapshot.detected_features.is_empty() {
                    unconfirmed
                } else {
                    snapshot.detected_features.join(", ")
                }
            )
        );
        for error in &snapshot.probe_errors {
            println!("{}", crate::t!("probe-note", error));
        }
        println!("{}", crate::t!("controls-not-enabled"));
        return;
    }
    if let Some(mode) = snapshot.noise_mode {
        println!("{}", crate::t!("probe-noise-mode-index", mode));
    } else {
        println!("{}", crate::t!("noise-mode-not-returned"));
    }
    println!(
        "{}",
        crate::t!(
            "probe-noise-mode-support",
            snapshot.noise_mode_support_note.as_str()
        )
    );
    if snapshot.supports_equalizer {
        if let Some(preset) = snapshot.equalizer_preset {
            let presets = crate::compatibility::equalizer_presets_for_product(snapshot.product_id);
            let preset_name = crate::compatibility::equalizer_preset_key(preset, presets)
                .map(|key| crate::t!(key))
                .unwrap_or_else(|| preset.to_string());
            println!("{}", crate::t!("probe-headset-eq", preset_name));
        } else {
            println!("{}", crate::t!("equalizer-not-returned"));
        }
    }
    if snapshot.supports_custom_equalizer {
        if !snapshot.custom_equalizers.is_empty() {
            let names: Vec<String> = snapshot
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
    if snapshot.supports_spatial_audio {
        if let Some(mode) = snapshot.spatial_audio_mode {
            let mode_key = crate::compatibility::spatial_audio_mode_key(mode);
            println!("{}", crate::t!("probe-spatial-audio", crate::t!(mode_key)));
        } else {
            println!("{}", crate::t!("spatial-audio-not-returned"));
        }
    }
    if snapshot.supports_game_mode {
        if let Some(enabled) = snapshot.game_mode_enabled {
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
    if snapshot.supports_bass_engine {
        if let Some(enabled) = snapshot.bass_engine_enabled {
            let state_str = if enabled {
                crate::t!("common-on")
            } else {
                crate::t!("common-off")
            };
            if let Some(level) = snapshot.bass_engine_level {
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
    if snapshot.supports_hi_res {
        if let Some(enabled) = snapshot.hi_res_enabled {
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
}

fn print_battery(state: &BatteryState) {
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
