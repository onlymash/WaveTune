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

//! Remote asset client and asynchronous downloader from encoapi.

use std::{sync::mpsc::Sender, thread, time::Duration};

use crate::app::events::UiUpdate;

use super::{
    cache::{
        generic_artwork_path, load_cached_artwork, load_cached_model, load_cached_scene,
        save_cached_artwork, save_cached_model, save_cached_scene,
    },
    vfxms,
};

static API_BASE_URL: &str = "https://encoapi.vercel.app";

pub fn spawn_artwork_fetch(product_id: u32, color_id: Option<u8>, updates_tx: Sender<UiUpdate>) {
    let cached_artwork = load_cached_artwork(product_id, color_id);
    let cached_model = load_cached_model(product_id, color_id);
    let cached_scene = load_cached_scene(product_id, color_id);

    if let Some(ref model_path) = cached_model {
        let _ = updates_tx.send(UiUpdate::DeviceModel(model_path.clone()));
    } else if let Some(ref scene_path) = cached_scene {
        let _ = updates_tx.send(UiUpdate::DeviceScene(scene_path.clone()));
    }

    if let Some(ref artwork_path) = cached_artwork {
        let _ = updates_tx.send(UiUpdate::DeviceArtwork(artwork_path.clone()));
    } else if cached_model.is_none() && cached_scene.is_none() {
        let _ = updates_tx.send(UiUpdate::DeviceArtwork(generic_artwork_path(false)));
    }

    let need_artwork = cached_artwork.is_none();
    let need_model = cached_model.is_none() && cached_scene.is_none();

    if !need_artwork && !need_model {
        return;
    }

    let _ = thread::Builder::new()
        .name("enco-artwork".to_string())
        .spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
            {
                Ok(c) => c,
                Err(_) => return,
            };

            let mut colors_to_try = Vec::new();
            if let Some(c) = color_id {
                colors_to_try.push(c);
            }
            for fallback in [1, 2, 3, 0] {
                if !colors_to_try.contains(&fallback) {
                    colors_to_try.push(fallback);
                }
            }

            // 1. Fetch static artwork if needed
            if need_artwork {
                for &color in &colors_to_try {
                    let url = format!(
                        "{API_BASE_URL}/v1/models/{product_id:06X}/assets/device?color={color}&theme=light"
                    );
                    if let Ok(response) = client.get(&url).send() {
                        if response.status().is_success() {
                            if let Ok(bytes) = response.bytes() {
                                if !bytes.is_empty() {
                                    if let Ok(saved_path) =
                                        save_cached_artwork(product_id, Some(color), &bytes)
                                    {
                                        let _ = updates_tx.send(UiUpdate::DeviceArtwork(saved_path));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Fetch 360° turntable model or 3D scene if needed
            if need_model {
                for &color in &colors_to_try {
                    let config_url = format!(
                        "{API_BASE_URL}/v1/models/{product_id:06X}/files/model?color={color}&path=config.json"
                    );
                    let mut model_webp_path: Option<String> = None;
                    let mut model_scene_path: Option<String> = None;

                    if let Ok(resp) = client.get(&config_url).send() {
                        if resp.status().is_success() {
                            if let Ok(text) = resp.text() {
                                if let Some(pos) = text.find("\"modelWebp\"") {
                                    if let Some(path_pos) = text[pos..].find("\"path\"") {
                                        if let Some(sub) = text[pos + path_pos..].split('"').nth(3) {
                                            if !sub.trim().is_empty() {
                                                model_webp_path = Some(sub.trim().to_string());
                                            }
                                        }
                                    }
                                } else if let Some(pos) = text.find("\"model3D\"") {
                                    if let Some(path_pos) = text[pos..].find("\"path\"") {
                                        if let Some(sub) = text[pos + path_pos..].split('"').nth(3) {
                                            if !sub.trim().is_empty() {
                                                model_scene_path = Some(sub.trim().to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(ref rel_path) = model_webp_path {
                        let model_url = format!(
                            "{API_BASE_URL}/v1/models/{product_id:06X}/files/model?color={color}&path={model_path}",
                            model_path = rel_path
                        );
                        if let Ok(resp) = client.get(&model_url).send() {
                            if resp.status().is_success() {
                                if let Ok(bytes) = resp.bytes() {
                                    if bytes.len() >= 12
                                        && &bytes[0..4] == b"RIFF"
                                        && &bytes[8..12] == b"WEBP"
                                    {
                                        if let Ok(saved_model) =
                                            save_cached_model(product_id, Some(color), &bytes)
                                        {
                                            let _ = updates_tx.send(UiUpdate::DeviceModel(saved_model));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(ref rel_path) = model_scene_path {
                        let download_url = format!(
                            "{API_BASE_URL}/v1/models/{product_id:06X}/download/model?color={color}"
                        );
                        if let Ok(resp) = client.get(&download_url).send() {
                            if resp.status().is_success() {
                                if let Ok(bytes) = resp.bytes() {
                                    if let Ok(mut archive) =
                                        zip::ZipArchive::new(std::io::Cursor::new(bytes))
                                    {
                                        use std::io::Read;
                                        let mut vfx_bytes = Vec::new();
                                        let mut found = false;
                                        if let Ok(mut file) = archive.by_name(rel_path) {
                                            if file.read_to_end(&mut vfx_bytes).is_ok() {
                                                found = true;
                                            }
                                        }
                                        if !found {
                                            for i in 0..archive.len() {
                                                if let Ok(mut file) = archive.by_index(i) {
                                                    if file
                                                        .name()
                                                        .is_ok_and(|name| name.ends_with(".vfxms"))
                                                    {
                                                        if file.read_to_end(&mut vfx_bytes).is_ok()
                                                        {
                                                            found = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if found && vfxms::VfxScene::parse(&vfx_bytes).is_ok() {
                                            if let Ok(saved_scene) =
                                                save_cached_scene(product_id, Some(color), &vfx_bytes)
                                            {
                                                let _ = updates_tx
                                                    .send(UiUpdate::DeviceScene(saved_scene));
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
}

