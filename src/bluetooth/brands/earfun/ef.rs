use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use crate::{
    app::events::{SettingDomain, UiUpdate, WorkerCommand},
    bluetooth::{
        ControlsSnapshot, DeviceChoice, DeviceIdentitySnapshot, DeviceSnapshot, DualDeviceSnapshot,
        NoiseSnapshot, ProbeDiagnostics, SoundSnapshot, WearSnapshot,
    },
    compatibility,
    protocol::{
        BatteryPart, BatteryState, FindEarbudsMode, FirmwareVersions,
        brands::earfun::{EarFunEfCommands, EarFunEfFrameDecoder, EarFunEfPacket},
    },
};

const EF_REQUEST_TIMEOUT: Duration = Duration::from_millis(1200);
const EF_PROBE_TIMEOUT: Duration = Duration::from_millis(700);
const EF_RETRY_DELAY: Duration = Duration::from_millis(200);

pub fn probe_firmware<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
) -> Option<Vec<u8>> {
    request(
        stream,
        decoder,
        EarFunEfCommands::FIRMWARE,
        &[],
        EF_PROBE_TIMEOUT,
    )
    .ok()
    .map(|packet| packet.payload)
}

pub fn initialize<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    device: &DeviceChoice,
    firmware_payload: Vec<u8>,
) -> Result<DeviceSnapshot, String> {
    let firmware_string = parse_firmware_string(&firmware_payload)?;
    let firmware_model_code = super::init::parse_earfun_model_code_from_firmware(&firmware_string);
    let profile = compatibility::profile_for_identity(
        device.product_id.unwrap_or_default(),
        firmware_model_code,
    );
    let read_only = profile.is_none_or(|profile| !profile.supports_controls());
    let product_id = device.product_id.unwrap_or_default();
    let has_case_battery = compatibility::earfun_has_case_battery(product_id, firmware_model_code);
    let is_headphones = compatibility::earfun_is_headphones(product_id, firmware_model_code);
    let mut diagnostics = ProbeDiagnostics::default();
    diagnostics
        .detected_features
        .push("ef-protocol".to_string());
    diagnostics
        .detected_features
        .push("firmware-version".to_string());

    let mut battery = if profile.is_some() {
        let battery_payload = request(
            stream,
            decoder,
            EarFunEfCommands::BATTERY,
            &[],
            EF_REQUEST_TIMEOUT,
        )?
        .payload;
        diagnostics.detected_features.push("battery".to_string());
        parse_battery(&battery_payload, is_headphones)
    } else {
        BatteryState::default()
    };
    if profile.is_some()
        && !is_headphones
        && let Some(packet) = optional_request(
            stream,
            decoder,
            EarFunEfCommands::CHARGING,
            "charging status",
            &mut diagnostics,
        )
        && let Some(flags) = packet.payload.first().copied()
        && flags & !0x0A == 0
    {
        set_charging(&mut battery.left, flags & 0x02 != 0);
        set_charging(&mut battery.right, flags & 0x08 != 0);
    }
    let mut case = if profile.is_some() && has_case_battery {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::CASE_BATTERY,
            "case battery",
            &mut diagnostics,
        )
        .and_then(|packet| parse_battery_component(packet.payload.first().copied()))
    } else {
        None
    };
    if let Some(case) = &mut case
        && let Some(packet) = optional_request(
            stream,
            decoder,
            EarFunEfCommands::CASE_CHARGING,
            "case charging status",
            &mut diagnostics,
        )
        && let Some(value) = packet.payload.first().filter(|value| **value <= 1)
    {
        case.charging = Some(*value != 0);
    }

    let firmware_version = firmware_string
        .split('_')
        .next_back()
        .unwrap_or(&firmware_string)
        .trim_start_matches('v')
        .to_string();
    let supported_indices = profile.map_or_else(Vec::new, |p| {
        p.noise_modes.iter().map(|mode| mode.index).collect()
    });
    let noise = if profile.is_some_and(|p| p.supports_noise_control()) {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::NOISE.0,
            "noise mode",
            &mut diagnostics,
        )
        .and_then(|packet| packet.payload.first().copied())
        .filter(|mode| supported_indices.contains(mode))
    } else {
        None
    };
    let game_mode = if profile.is_some_and(|p| p.has_feature(compatibility::features::GAME_MODE)) {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::GAME.0,
            "game mode",
            &mut diagnostics,
        )
        .and_then(|packet| packet.payload.first().copied())
        .filter(|value| matches!(value, 1 | 2))
        .map(|value| value == 2)
    } else {
        None
    };
    let prompt_volume = if profile.is_some_and(|p| p.supports_prompt_volume()) {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::PROMPT_VOLUME.0,
            "prompt volume",
            &mut diagnostics,
        )
        .and_then(|packet| packet.payload.first().copied())
        .filter(|value| (1..=5).contains(value))
        .map(|value| 25 * (value - 1))
    } else {
        None
    };
    let spatial_audio_mode = if profile.is_some_and(|p| p.supports_spatial_audio()) {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::SPATIAL_AUDIO.0,
            "spatial audio mode",
            &mut diagnostics,
        )
        .and_then(|packet| packet.payload.first().copied())
        .filter(|value| matches!(value, 1 | 2))
        .map(|value| u8::from(value == 2))
    } else {
        None
    };
    let find_mode = if profile.is_some_and(|p| p.supports_find_device()) {
        optional_request(
            stream,
            decoder,
            EarFunEfCommands::FIND_DEVICE.0,
            "find-device state",
            &mut diagnostics,
        )
        .and_then(|packet| parse_find_mode(&packet.payload))
    } else {
        None
    };

    let battery = BatteryState {
        left: battery.left,
        right: battery.right,
        case,
    };
    Ok(DeviceSnapshot {
        identity: DeviceIdentitySnapshot {
            name: device.name.clone(),
            model: profile.map_or_else(
                || crate::t!("device-unknown-model"),
                |profile| profile.model.to_string(),
            ),
            firmware_model_code: firmware_model_code.map(str::to_string),
            product_id,
            color_id: None,
            is_known_model: profile.is_some(),
            read_only,
            firmware: FirmwareVersions {
                left: Some(firmware_version.clone()),
                right: (!is_headphones).then_some(firmware_version),
                case: None,
            },
        },
        battery,
        wear: WearSnapshot::default(),
        noise: NoiseSnapshot {
            current_mode: noise,
            supported_indices,
            modes: profile.map_or(&[], |profile| profile.noise_modes),
            support_note: String::new(),
            anc_sub: 0,
            transp_sub: 0,
        },
        sound: SoundSnapshot {
            supports_game_mode: profile
                .is_some_and(|p| p.has_feature(compatibility::features::GAME_MODE)),
            game_mode_enabled: game_mode,
            supports_spatial_audio: profile.is_some_and(|p| p.supports_spatial_audio()),
            spatial_audio_mode,
            ..SoundSnapshot::default()
        },
        controls: ControlsSnapshot {
            supports_find_device: profile.is_some_and(|p| p.supports_find_device()),
            supports_find_device_sides: false,
            supports_prompt_volume: profile.is_some_and(|p| p.supports_prompt_volume()),
            prompt_volume,
            prompt_volume_range: profile
                .is_some_and(|p| p.supports_prompt_volume())
                .then_some((0, 100)),
            find_earbuds_mode: find_mode,
            is_finding_earbuds: find_mode.is_some_and(FindEarbudsMode::is_finding),
            ..ControlsSnapshot::default()
        },
        dual_device: DualDeviceSnapshot::default(),
        diagnostics,
    })
}

pub fn poll<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    snapshot: &DeviceSnapshot,
    updates: &Sender<UiUpdate>,
) -> Result<(), String> {
    if snapshot.identity.read_only {
        return Ok(());
    }
    let is_headphones = compatibility::earfun_is_headphones(
        snapshot.identity.product_id,
        snapshot.identity.firmware_model_code.as_deref(),
    );
    let battery_payload = request(
        stream,
        decoder,
        EarFunEfCommands::BATTERY,
        &[],
        EF_REQUEST_TIMEOUT,
    )?
    .payload;
    let mut battery = parse_battery(&battery_payload, is_headphones);
    if !is_headphones
        && let Ok(packet) = request(
            stream,
            decoder,
            EarFunEfCommands::CHARGING,
            &[],
            EF_REQUEST_TIMEOUT,
        )
        && let Some(flags) = packet.payload.first().copied()
        && flags & !0x0A == 0
    {
        set_charging(&mut battery.left, flags & 0x02 != 0);
        set_charging(&mut battery.right, flags & 0x08 != 0);
    }
    let mut case = if compatibility::earfun_has_case_battery(
        snapshot.identity.product_id,
        snapshot.identity.firmware_model_code.as_deref(),
    ) {
        request(
            stream,
            decoder,
            EarFunEfCommands::CASE_BATTERY,
            &[],
            EF_REQUEST_TIMEOUT,
        )
        .ok()
        .and_then(|packet| parse_battery_component(packet.payload.first().copied()))
    } else {
        None
    };
    if let Some(case) = &mut case
        && let Ok(packet) = request(
            stream,
            decoder,
            EarFunEfCommands::CASE_CHARGING,
            &[],
            EF_REQUEST_TIMEOUT,
        )
        && let Some(value) = packet.payload.first().filter(|value| **value <= 1)
    {
        case.charging = Some(*value != 0);
    }
    let _ = updates.send(UiUpdate::Battery(BatteryState {
        left: battery.left,
        right: battery.right,
        case,
    }));

    if !snapshot.noise.supported_indices.is_empty()
        && let Ok(packet) = request(
            stream,
            decoder,
            EarFunEfCommands::NOISE.0,
            &[],
            EF_REQUEST_TIMEOUT,
        )
        && let Some(mode) = packet.payload.first()
        && snapshot.noise.supported_indices.contains(mode)
    {
        let _ = updates.send(UiUpdate::NoiseMode(*mode));
    }
    Ok(())
}

pub fn handle_command<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    snapshot: &mut DeviceSnapshot,
    command: WorkerCommand,
    updates: &Sender<UiUpdate>,
) -> Result<(), String> {
    let profile = compatibility::profile_for_identity(
        snapshot.identity.product_id,
        snapshot.identity.firmware_model_code.as_deref(),
    );
    match command {
        WorkerCommand::SetNoiseMode(mode) => {
            if snapshot.identity.read_only || !snapshot.noise.supported_indices.contains(&mode) {
                send_setting_error(
                    updates,
                    SettingDomain::NoiseMode,
                    crate::t!("noise-mode-blocked"),
                );
            } else if set_and_confirm(
                stream,
                decoder,
                EarFunEfCommands::NOISE,
                &[mode],
                |payload| payload.first() == Some(&mode),
            )
            .is_ok()
            {
                let _ = updates.send(UiUpdate::NoiseMode(mode));
                let _ = updates.send(UiUpdate::NoiseModeWriteCompleted(mode));
            } else {
                send_setting_error(
                    updates,
                    SettingDomain::NoiseMode,
                    crate::t!("noise-mode-switch-failed", "EF readback did not match"),
                );
            }
        }
        WorkerCommand::SetGameMode(enabled) => {
            if snapshot.identity.read_only
                || !profile.is_some_and(|p| p.has_feature(compatibility::features::GAME_MODE))
            {
                send_setting_error(
                    updates,
                    SettingDomain::GameMode,
                    crate::t!("game-mode-blocked"),
                );
            } else {
                let value = if enabled { 2 } else { 1 };
                match set_and_confirm(
                    stream,
                    decoder,
                    EarFunEfCommands::GAME,
                    &[value],
                    |payload| payload.first() == Some(&value),
                ) {
                    Ok(_) => {
                        if let Some(sound) = &mut snapshot.sound.game_mode_enabled {
                            *sound = enabled;
                        } else {
                            snapshot.sound.game_mode_enabled = Some(enabled);
                        }
                        let _ = updates.send(UiUpdate::GameMode(enabled));
                    }
                    Err(error) => {
                        send_setting_error(
                            updates,
                            SettingDomain::GameMode,
                            crate::t!("game-mode-switch-failed", error),
                        );
                    }
                }
            }
        }
        WorkerCommand::SetSpatialAudio(mode) => {
            if snapshot.identity.read_only
                || !profile.is_some_and(|p| p.supports_spatial_audio())
                || mode > 1
            {
                send_setting_error(
                    updates,
                    SettingDomain::SpatialAudio,
                    crate::t!("spatial-audio-blocked"),
                );
            } else {
                let value = if mode == 0 { 1 } else { 2 };
                match set_and_confirm(
                    stream,
                    decoder,
                    EarFunEfCommands::SPATIAL_AUDIO,
                    &[value],
                    |payload| payload.first() == Some(&value),
                ) {
                    Ok(_) => {
                        snapshot.sound.spatial_audio_mode = Some(mode);
                        let _ = updates.send(UiUpdate::SpatialAudio(mode));
                    }
                    Err(error) => {
                        send_setting_error(
                            updates,
                            SettingDomain::SpatialAudio,
                            crate::t!("spatial-audio-switch-failed", error),
                        );
                    }
                }
            }
        }
        WorkerCommand::SetPromptVolume(volume) => {
            if snapshot.identity.read_only
                || !profile.is_some_and(|p| p.supports_prompt_volume())
                || volume > 100
                || volume % 25 != 0
            {
                send_setting_error(
                    updates,
                    SettingDomain::Controls,
                    crate::t!("controls-prompt-volume-blocked"),
                );
            } else {
                let value = 1 + volume / 25;
                match set_and_confirm(
                    stream,
                    decoder,
                    EarFunEfCommands::PROMPT_VOLUME,
                    &[value],
                    |payload| payload.first() == Some(&value),
                ) {
                    Ok(_) => {
                        snapshot.controls.prompt_volume = Some(volume);
                        let _ = updates.send(UiUpdate::PromptVolume(volume));
                    }
                    Err(error) => {
                        send_setting_error(
                            updates,
                            SettingDomain::Controls,
                            crate::t!("controls-prompt-volume-update-failed", error),
                        );
                    }
                }
            }
        }
        WorkerCommand::FindEarbuds(mode) => {
            if snapshot.identity.read_only
                || !profile.is_some_and(|p| p.supports_find_device())
                || matches!(mode, FindEarbudsMode::Left | FindEarbudsMode::Right)
            {
                let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                    requested: mode,
                    actual: snapshot.controls.find_earbuds_mode,
                });
                send_setting_error(
                    updates,
                    SettingDomain::Controls,
                    crate::t!("controls-find-blocked"),
                );
            } else {
                let ringing = mode == FindEarbudsMode::Both;
                let payload = if ringing { [1, 1] } else { [0, 0] };
                match set_and_confirm(
                    stream,
                    decoder,
                    EarFunEfCommands::FIND_DEVICE,
                    &payload,
                    |report| parse_find_mode(report) == Some(mode),
                ) {
                    Ok(_) => {
                        snapshot.controls.find_earbuds_mode = Some(mode);
                        snapshot.controls.is_finding_earbuds = ringing;
                        let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                            requested: mode,
                            actual: Some(mode),
                        });
                    }
                    Err(error) => {
                        send_setting_error(
                            updates,
                            SettingDomain::Controls,
                            crate::t!("controls-find-failed", error),
                        );
                        let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                            requested: mode,
                            actual: snapshot.controls.find_earbuds_mode,
                        });
                    }
                }
            }
        }
        WorkerCommand::Disconnect => {}
        WorkerCommand::SetEqualizer(_)
        | WorkerCommand::SelectCustomEqualizer(_)
        | WorkerCommand::AddCustomEqualizer { .. }
        | WorkerCommand::SaveCustomEqualizer { .. }
        | WorkerCommand::DeleteCustomEqualizer(_) => send_setting_error(
            updates,
            SettingDomain::Equalizer,
            crate::t!("earfun-ef-feature-not-implemented"),
        ),
        WorkerCommand::SetBassEngineEnabled(_) | WorkerCommand::SetBassEngineLevel(_) => {
            send_setting_error(
                updates,
                SettingDomain::BassEngine,
                crate::t!("earfun-ef-feature-not-implemented"),
            )
        }
        WorkerCommand::SetHiRes(_) => send_setting_error(
            updates,
            SettingDomain::HiRes,
            crate::t!("earfun-ef-feature-not-implemented"),
        ),
        WorkerCommand::SetAudioCodec(_) => send_setting_error(
            updates,
            SettingDomain::AudioCodec,
            crate::t!("earfun-ef-feature-not-implemented"),
        ),
        WorkerCommand::SetGesture { .. }
        | WorkerCommand::SetNoiseCycle { .. }
        | WorkerCommand::RunFitTest
        | WorkerCommand::SetMicrophoneMode(_)
        | WorkerCommand::SetHeadMotionType(_)
        | WorkerCommand::SetDualDeviceConnection(_)
        | WorkerCommand::SetInEarDetection(_)
        | WorkerCommand::SetTouchLock(_)
        | WorkerCommand::SetSleepDetection(_)
        | WorkerCommand::SetPreferredDevice(_)
        | WorkerCommand::SetPairedDevice { .. } => send_setting_error(
            updates,
            SettingDomain::Controls,
            crate::t!("earfun-ef-feature-not-implemented"),
        ),
    }
    Ok(())
}

pub fn handle_incoming_bytes(
    decoder: &mut EarFunEfFrameDecoder,
    snapshot: &mut Option<DeviceSnapshot>,
    bytes: &[u8],
    updates: &Sender<UiUpdate>,
) {
    for packet in decoder.push(bytes) {
        let Some(snapshot) = snapshot.as_mut() else {
            continue;
        };
        match packet.command {
            command if command == EarFunEfCommands::NOISE.0 => {
                if let Some(mode) = packet.payload.first().copied()
                    && snapshot.noise.supported_indices.contains(&mode)
                {
                    snapshot.noise.current_mode = Some(mode);
                    let _ = updates.send(UiUpdate::NoiseMode(mode));
                }
            }
            command if command == EarFunEfCommands::GAME.0 => {
                if let Some(enabled) = packet
                    .payload
                    .first()
                    .filter(|value| matches!(value, 1 | 2))
                    .map(|value| *value == 2)
                {
                    snapshot.sound.game_mode_enabled = Some(enabled);
                    let _ = updates.send(UiUpdate::GameMode(enabled));
                }
            }
            command if command == EarFunEfCommands::PROMPT_VOLUME.0 => {
                if let Some(volume) = packet
                    .payload
                    .first()
                    .filter(|value| (1..=5).contains(*value))
                    .map(|value| 25 * (*value - 1))
                {
                    snapshot.controls.prompt_volume = Some(volume);
                    let _ = updates.send(UiUpdate::PromptVolume(volume));
                }
            }
            command if command == EarFunEfCommands::SPATIAL_AUDIO.0 => {
                if let Some(mode) = packet
                    .payload
                    .first()
                    .filter(|value| matches!(value, 1 | 2))
                    .map(|value| u8::from(*value == 2))
                {
                    snapshot.sound.spatial_audio_mode = Some(mode);
                    let _ = updates.send(UiUpdate::SpatialAudio(mode));
                }
            }
            command if command == EarFunEfCommands::FIND_DEVICE.0 => {
                if let Some(mode) = parse_find_mode(&packet.payload) {
                    snapshot.controls.find_earbuds_mode = Some(mode);
                    snapshot.controls.is_finding_earbuds = mode.is_finding();
                    let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                        requested: mode,
                        actual: Some(mode),
                    });
                }
            }
            _ => {}
        }
    }
}

fn request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    command: u16,
    payload: &[u8],
    timeout: Duration,
) -> Result<EarFunEfPacket, String> {
    let packet =
        EarFunEfPacket::new(command, payload.to_vec()).map_err(|error| error.to_string())?;
    let frame = packet.encode().map_err(|error| error.to_string())?;
    stream
        .write_all(&frame)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        let mut bytes = [0; 512];
        match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(length) => {
                if let Some(response) = decoder
                    .push(&bytes[..length])
                    .into_iter()
                    .find(|response| response.command == command)
                {
                    return Ok(response);
                }
            }
            Err(error) if crate::bluetooth::transport::is_socket_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(crate::t!(
                        "protocol-query-timeout",
                        format!("{command:04X}")
                    ));
                }
            }
            Err(error) => return Err(crate::t!("protocol-response-read-failed", error)),
        }
        if Instant::now() >= deadline {
            return Err(crate::t!(
                "protocol-query-timeout",
                format!("{command:04X}")
            ));
        }
    }
}

fn optional_request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    command: u16,
    feature: &str,
    diagnostics: &mut ProbeDiagnostics,
) -> Option<EarFunEfPacket> {
    match request(stream, decoder, command, &[], EF_REQUEST_TIMEOUT) {
        Ok(packet) => Some(packet),
        Err(error) => {
            diagnostics
                .probe_errors
                .push(format!("EF {feature} query failed: {error}"));
            None
        }
    }
}

fn set_and_confirm<T, F>(
    stream: &mut T,
    decoder: &mut EarFunEfFrameDecoder,
    commands: (u16, u16),
    payload: &[u8],
    matches: F,
) -> Result<(), String>
where
    T: Read + Write,
    F: Fn(&[u8]) -> bool,
{
    let frame = EarFunEfPacket::new(commands.1, payload.to_vec())
        .and_then(|packet| packet.encode())
        .map_err(|error| error.to_string())?;
    stream
        .write_all(&frame)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match request(stream, decoder, commands.0, &[], EF_RETRY_DELAY) {
            Ok(response) if matches(&response.payload) => return Ok(()),
            Ok(_) if Instant::now() < deadline => {
                std::thread::sleep(EF_RETRY_DELAY);
            }
            Err(error) if error == crate::t!("channel-headset-closed") => return Err(error),
            Err(error)
                if error != crate::t!("protocol-query-timeout", format!("{:04X}", commands.0)) =>
            {
                return Err(error);
            }
            Err(_) if Instant::now() < deadline => {
                std::thread::sleep(EF_RETRY_DELAY);
            }
            Ok(_) | Err(_) => {
                return Err(crate::t!(
                    "protocol-query-timeout",
                    format!("{:04X}", commands.0)
                ));
            }
        }
    }
}

fn parse_firmware_string(payload: &[u8]) -> Result<String, String> {
    if payload.is_empty() || payload.len() > 128 {
        return Err(crate::t!(
            "protocol-response-read-failed",
            "invalid EF firmware response"
        ));
    }
    if payload
        .iter()
        .all(|byte| (0x20..=0x7E).contains(byte) || *byte == 0)
    {
        let value = String::from_utf8_lossy(payload)
            .trim_matches(char::from(0))
            .trim()
            .to_string();
        if value.is_empty() {
            return Err(crate::t!(
                "protocol-response-read-failed",
                "empty EF firmware response"
            ));
        }
        Ok(value)
    } else if payload.len() == 3 {
        Ok(format!("{}.{}.{}", payload[0], payload[1], payload[2]))
    } else {
        Err(crate::t!(
            "protocol-response-read-failed",
            "invalid EF firmware response"
        ))
    }
}

fn parse_battery(payload: &[u8], headphones: bool) -> BatteryState {
    let left = parse_battery_component(payload.first().copied());
    let right = if headphones {
        None
    } else {
        parse_battery_component(payload.get(1).copied())
    };
    BatteryState {
        left,
        right,
        case: None,
    }
}

fn parse_battery_component(value: Option<u8>) -> Option<BatteryPart> {
    value
        .filter(|value| *value <= 100)
        .map(|percentage| BatteryPart {
            percentage,
            charging: None,
        })
}

fn set_charging(part: &mut Option<BatteryPart>, charging: bool) {
    if let Some(part) = part {
        part.charging = Some(charging);
    }
}

fn parse_find_mode(payload: &[u8]) -> Option<FindEarbudsMode> {
    match payload {
        [left, right, ..] if *left <= 1 && *right <= 1 => Some(match (*left, *right) {
            (0, 0) => FindEarbudsMode::Off,
            (1, 0) => FindEarbudsMode::Left,
            (0, 1) => FindEarbudsMode::Right,
            (1, 1) => FindEarbudsMode::Both,
            _ => unreachable!(),
        }),
        _ => None,
    }
}

fn send_setting_error(updates: &Sender<UiUpdate>, domain: SettingDomain, message: String) {
    let _ = updates.send(UiUpdate::SettingError { domain, message });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bluetooth::{DeviceBrand, DeviceChoice};

    struct NoIo {
        writes: Vec<u8>,
    }

    impl Read for NoIo {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("unknown EF models must not trigger status queries");
        }
    }

    impl Write for NoIo {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.writes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn unknown_ef_firmware_stays_read_only_without_matching_device_name() {
        let device = DeviceChoice {
            address: bluer::Address::new([0; 6]),
            name: "EarFun Air Pro 4i".to_string(),
            brand: DeviceBrand::EarFun,
            product_id: None,
            has_control_service: true,
            control_service_uuids: Vec::new(),
            has_earfun_gaia_vendor_service: false,
        };
        let mut stream = NoIo { writes: Vec::new() };
        let snapshot = initialize(
            &mut stream,
            &mut EarFunEfFrameDecoder::default(),
            &device,
            b"unrecognized-firmware".to_vec(),
        )
        .unwrap();

        assert!(snapshot.identity.read_only);
        assert!(!snapshot.identity.is_known_model);
        assert!(stream.writes.is_empty());
    }
}
