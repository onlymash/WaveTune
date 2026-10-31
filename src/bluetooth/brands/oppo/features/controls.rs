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

use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
};

use crate::app::events::{SettingDomain, UiUpdate, WorkerCommand};
use crate::bluetooth::brands::oppo::transport::request;
use crate::protocol::{
    EarSide, FrameDecoder, KeyFunction, NoiseCycleSetting, NoiseCycleState, WearState,
    encode_find_earbuds, encode_fit_test, encode_set_head_motion_type, encode_set_key_function,
    encode_set_noise_cycle, encode_set_prompt_volume, parse_feature_flags_response,
    parse_head_motion_type_response, parse_noise_cycle_response, parse_prompt_volume_response,
    parse_touch_controls_response,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_controls_command<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    command: &WorkerCommand,
    read_only: bool,
    supports_touch_customization: bool,
    current_key_functions: &mut Vec<KeyFunction>,
    noise_modes_empty: bool,
    current_noise_cycle: &mut NoiseCycleState,
    supports_find_device: bool,
    is_finding_earbuds: &mut bool,
    supports_fit_detection: bool,
    current_wear: &WearState,
    supports_prompt_volume: bool,
    current_prompt_volume: &mut Option<u8>,
    supports_head_motion: bool,
    current_head_motion: &mut Option<u8>,
    supports_wear_detection: bool,
    current_in_ear_detection: &mut Option<bool>,
    supports_sleep_detection: bool,
    current_sleep_detection: &mut Option<bool>,
    updates: &Sender<UiUpdate>,
) -> bool {
    match command {
        WorkerCommand::SetGesture {
            side,
            button,
            action,
            function,
        } => {
            if read_only || !supports_touch_customization {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-gesture-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("controls-gesture-updating")));
            let key_function = KeyFunction {
                side: *side,
                button: *button,
                action: *action,
                function: *function,
            };
            let payload = encode_set_key_function(&key_function);
            match request(stream, decoder, 0x0401, 0, payload, Some(updates)) {
                Ok(_) => {
                    let mut updated = false;
                    if let Ok(res) =
                        request(stream, decoder, 0x0108, 0, vec![2, 3, 1], Some(updates))
                    {
                        if let Ok(list) = parse_touch_controls_response(&res.payload) {
                            if !list.is_empty() {
                                *current_key_functions = list.clone();
                                let _ = updates.send(UiUpdate::KeyFunctions(list));
                                updated = true;
                            }
                        }
                    }
                    if !updated {
                        if let Some(existing) = current_key_functions.iter_mut().find(|k| {
                            (k.side == *side || k.side == EarSide::Both)
                                && k.button == *button
                                && k.action == *action
                        }) {
                            existing.function = *function;
                        } else {
                            current_key_functions.push(KeyFunction {
                                side: *side,
                                button: *button,
                                action: *action,
                                function: *function,
                            });
                        }
                        let _ = updates.send(UiUpdate::KeyFunctions(current_key_functions.clone()));
                    }
                    let _ = updates.send(UiUpdate::Status(crate::t!("controls-gesture-updated")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-gesture-update-failed", error),
                    });
                    let _ = updates.send(UiUpdate::KeyFunctions(current_key_functions.clone()));
                }
            }
            true
        }
        WorkerCommand::SetNoiseCycle {
            side,
            mask,
            is_shared,
        } => {
            if read_only || noise_modes_empty {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-noise-cycle-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("controls-noise-cycle-updating")));
            let setting = NoiseCycleSetting {
                side: *side,
                mask: *mask,
                is_shared: *is_shared,
            };
            let payload = encode_set_noise_cycle(&setting);
            match request(stream, decoder, 0x0404, 0, payload, Some(updates)) {
                Ok(_) => {
                    let cycle_type = if *is_shared {
                        1
                    } else if *side == EarSide::Left {
                        3
                    } else {
                        4
                    };
                    if let Ok(res) = request(
                        stream,
                        decoder,
                        0x010c,
                        0,
                        vec![2, cycle_type],
                        Some(updates),
                    ) {
                        if let Ok((_, new_mask)) = parse_noise_cycle_response(&res.payload) {
                            if *is_shared {
                                current_noise_cycle.left_mask = new_mask;
                                current_noise_cycle.right_mask = new_mask;
                            } else if *side == EarSide::Left {
                                current_noise_cycle.left_mask = new_mask;
                            } else {
                                current_noise_cycle.right_mask = new_mask;
                            }
                            let _ = updates.send(UiUpdate::NoiseCycle(*current_noise_cycle));
                        }
                    }
                    let _ =
                        updates.send(UiUpdate::Status(crate::t!("controls-noise-cycle-updated")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-noise-cycle-update-failed", error),
                    });
                    let _ = updates.send(UiUpdate::NoiseCycle(*current_noise_cycle));
                }
            }
            true
        }
        WorkerCommand::FindEarbuds(ring) => {
            if read_only || !supports_find_device {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-find-blocked"),
                });
                return true;
            }
            let payload = encode_find_earbuds(*ring);
            match request(stream, decoder, 0x0400, 0, payload, Some(updates)) {
                Ok(_) => {
                    *is_finding_earbuds = *ring;
                    let _ = updates.send(UiUpdate::IsFindingEarbuds(*ring));
                    let _ = updates.send(UiUpdate::Status(if *ring {
                        crate::t!("controls-find-ringing")
                    } else {
                        crate::t!("controls-find-stopped")
                    }));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-find-failed", error),
                    });
                    let _ = updates.send(UiUpdate::IsFindingEarbuds(*is_finding_earbuds));
                }
            }
            true
        }
        WorkerCommand::RunFitTest => {
            if read_only || !supports_fit_detection {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-fit-test-blocked"),
                });
                return true;
            }
            if current_wear.left.is_some_and(|p| !p.in_ear)
                || current_wear.right.is_some_and(|p| !p.in_ear)
            {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-fit-test-wear-both"),
                });
                return true;
            }
            let payload = encode_fit_test(true);
            match request(stream, decoder, 0x0405, 0, payload, Some(updates)) {
                Ok(_) => {
                    let _ = updates.send(UiUpdate::FitTestRunning(true));
                    let _ = updates.send(UiUpdate::Status(crate::t!("controls-fit-test-running")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-fit-test-start-failed", error),
                    });
                    let _ = updates.send(UiUpdate::FitTestRunning(false));
                }
            }
            true
        }
        WorkerCommand::SetPromptVolume(volume) => {
            if read_only || !supports_prompt_volume {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-prompt-volume-blocked"),
                });
                return true;
            }
            let payload = encode_set_prompt_volume(*volume);
            match request(stream, decoder, 0x0427, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) = request(stream, decoder, 0x0130, 0, vec![], Some(updates)) {
                        if let Ok(actual) = parse_prompt_volume_response(&res.payload) {
                            *current_prompt_volume = Some(actual);
                            let _ = updates.send(UiUpdate::PromptVolume(actual));
                        }
                    }
                    let _ = updates.send(UiUpdate::Status(crate::t!(
                        "controls-prompt-volume-updated"
                    )));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-prompt-volume-update-failed", error),
                    });
                    if let Some(v) = *current_prompt_volume {
                        let _ = updates.send(UiUpdate::PromptVolume(v));
                    }
                }
            }
            true
        }
        WorkerCommand::SetHeadMotionType(motion_type) => {
            if read_only || !supports_head_motion {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-head-motion-blocked"),
                });
                return true;
            }
            let payload = encode_set_head_motion_type(*motion_type);
            match request(stream, decoder, 0x0431, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) = request(stream, decoder, 0x0134, 0, vec![], Some(updates)) {
                        if let Ok(actual) = parse_head_motion_type_response(&res.payload) {
                            *current_head_motion = Some(actual);
                            let _ = updates.send(UiUpdate::HeadMotionType(actual));
                        }
                    }
                    let _ =
                        updates.send(UiUpdate::Status(crate::t!("controls-head-motion-updated")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-head-motion-update-failed", error),
                    });
                    if let Some(m) = *current_head_motion {
                        let _ = updates.send(UiUpdate::HeadMotionType(m));
                    }
                }
            }
            true
        }
        WorkerCommand::SetInEarDetection(enabled) => {
            if read_only || !supports_wear_detection {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("wear-detection-blocked"),
                });
                return true;
            }
            let payload = vec![0x04, if *enabled { 1 } else { 0 }];
            match request(stream, decoder, 0x0403, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) =
                        request(stream, decoder, 0x010d, 0, vec![1, 0x04], Some(updates))
                    {
                        if let Ok(actual) = parse_feature_flags_response(&res.payload, 0x04) {
                            *current_in_ear_detection = Some(actual);
                            let _ = updates.send(UiUpdate::InEarDetectionEnabled(actual));
                        }
                    }
                    let _ = updates.send(UiUpdate::Status(crate::t!("wear-detection-updated")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("wear-detection-update-failed", error),
                    });
                    if let Some(w) = *current_in_ear_detection {
                        let _ = updates.send(UiUpdate::InEarDetectionEnabled(w));
                    }
                }
            }
            true
        }
        WorkerCommand::SetSleepDetection(enabled) => {
            if read_only || !supports_sleep_detection {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-sleep-detection-blocked"),
                });
                return true;
            }
            let payload = vec![0x3a, if *enabled { 1 } else { 0 }];
            match request(stream, decoder, 0x0403, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) =
                        request(stream, decoder, 0x010d, 0, vec![1, 0x3a], Some(updates))
                    {
                        if let Ok(actual) = parse_feature_flags_response(&res.payload, 0x3a) {
                            *current_sleep_detection = Some(actual);
                            let _ = updates.send(UiUpdate::SleepDetectionEnabled(actual));
                        }
                    }
                    let _ = updates.send(UiUpdate::Status(crate::t!(
                        "controls-sleep-detection-updated"
                    )));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-sleep-detection-update-failed", error),
                    });
                    if let Some(s) = *current_sleep_detection {
                        let _ = updates.send(UiUpdate::SleepDetectionEnabled(s));
                    }
                }
            }
            true
        }
        _ => false,
    }
}
