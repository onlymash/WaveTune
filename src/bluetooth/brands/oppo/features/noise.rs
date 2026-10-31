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
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

use crate::app::events::{SettingDomain, UiUpdate, WorkerCommand};
use crate::compatibility::{self, NoiseMode};
use crate::protocol::{FrameDecoder, WearState, encode_noise_mode, parse_noise_mode_response};

use crate::bluetooth::brands::oppo::features::OppoFeatureContext;
use crate::bluetooth::brands::oppo::transport::request;
use crate::bluetooth::brands::oppo::unsolicited::merge_wear_state;

#[allow(dead_code)]
pub fn noise_mode_name(index: u8, modes: &[NoiseMode]) -> String {
    compatibility::noise_mode_for(modes, index)
        .map(|mode| crate::t!(mode.kind.as_str()))
        .unwrap_or_else(|| crate::t!("noise-mode-protocol-mode", index))
}

pub fn set_noise_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    supported_indices: &[u8],
    modes: &[NoiseMode],
    current_wear: &mut WearState,
    supports_wear_detection: bool,
    protocol_index: u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    if supports_wear_detection && !current_wear.any_earbud_in_ear() {
        return Err(crate::t!("wear-earbuds-not-in-ear"));
    }
    let (deferred_tx, deferred_rx) = mpsc::channel();
    let result = (|| {
        if !compatibility::noise_mode_supported_for(protocol_index, supported_indices, modes) {
            return Err(crate::t!(
                "noise-mode-not-in-supported-list",
                protocol_index
            ));
        }
        if compatibility::noise_mode_for(modes, protocol_index).is_none() {
            return Err(crate::t!(
                "noise-mode-not-enabled-in-profile",
                protocol_index
            ));
        }
        let payload = encode_noise_mode(protocol_index);
        request(stream, decoder, 0x0404, 0, payload, Some(&deferred_tx))?;

        let result = request(stream, decoder, 0x010c, 0, vec![1, 1], Some(&deferred_tx))?;
        let mut actual = parse_noise_mode_response(&result.payload)
            .ok_or_else(|| crate::t!("noise-mode-readback-unrecognized"))?;
        if !compatibility::noise_mode_readback_matches_for(protocol_index, actual, modes) {
            for delay_ms in [250, 500] {
                thread::sleep(Duration::from_millis(delay_ms));
                if let Ok(retry) =
                    request(stream, decoder, 0x010c, 0, vec![1, 1], Some(&deferred_tx))
                {
                    if let Some(retry_val) = parse_noise_mode_response(&retry.payload) {
                        if compatibility::noise_mode_readback_matches_for(
                            protocol_index,
                            retry_val,
                            modes,
                        ) {
                            actual = retry_val;
                            break;
                        }
                    }
                }
            }
        }
        if !compatibility::noise_mode_readback_matches_for(protocol_index, actual, modes) {
            return Err(crate::t!(
                "noise-mode-readback-mismatch",
                actual,
                protocol_index
            ));
        }
        Ok(actual)
    })();
    for update in deferred_rx.try_iter() {
        match update {
            UiUpdate::Wear(ref wear) => merge_wear_state(current_wear, wear.clone()),
            UiUpdate::NoiseMode(_) => continue,
            _ => {}
        }
        let _ = updates.send(update);
    }
    result
}

pub fn handle_noise_command<T: Read + Write>(
    ctx: &mut OppoFeatureContext<'_, T>,
    command: &WorkerCommand,
) -> bool {
    match command {
        WorkerCommand::SetNoiseMode(protocol_index) => {
            let modes = ctx.noise_modes();
            if ctx.read_only {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::NoiseMode,
                    message: crate::t!("noise-mode-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("noise-mode-waiting-confirm")));
            let result = set_noise_mode(
                ctx.stream,
                ctx.decoder,
                ctx.supported_indices,
                modes,
                ctx.wear,
                ctx.supports_wear_detection,
                *protocol_index,
                ctx.updates,
            );
            match result {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::NoiseMode(actual));
                    let _ = ctx.updates.send(UiUpdate::Status(crate::t!(
                        "noise-mode-confirmed-and-readback"
                    )));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::NoiseMode,
                        message: error,
                    });
                }
            }
            let _ = ctx.updates.send(UiUpdate::NoiseControlsEnabled(
                !ctx.read_only && !modes.is_empty(),
            ));
            true
        }
        _ => false,
    }
}
