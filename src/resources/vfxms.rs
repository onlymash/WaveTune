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

use serde::{Deserialize, Serialize};

pub const HEADER_SIZE: usize = 40;
pub const GLB_MAGIC: &[u8; 4] = b"glTF";

#[derive(Debug, PartialEq, Eq)]
pub enum VfxmsError {
    TooShort,
    InvalidSectionLayout,
    SectionExceedsFile,
    MissingRequiredSection,
    InvalidGlbMagic,
    InvalidJson(String),
}

impl std::fmt::Display for VfxmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, ".vfxms file is too short (minimum 40 bytes)"),
            Self::InvalidSectionLayout => write!(f, "Invalid .vfxms section layout"),
            Self::SectionExceedsFile => write!(f, ".vfxms section exceeds file length"),
            Self::MissingRequiredSection => {
                write!(f, ".vfxms missing required config or glb section")
            }
            Self::InvalidGlbMagic => {
                write!(f, "Embedded model section does not start with glTF magic")
            }
            Self::InvalidJson(err) => write!(f, "Invalid .vfxms config JSON: {err}"),
        }
    }
}

impl std::error::Error for VfxmsError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VfxMaterialParameter {
    pub name: String,
    pub value: f32,
    pub color: [f32; 4],
    pub is_color: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VfxMaterialConfig {
    pub name: String,
    pub parameters: Vec<VfxMaterialParameter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VfxSceneConfig {
    pub name: String,
    pub anti_aliasing: i32,
    pub enable_skybox: bool,
    pub enable_bloom: bool,
    pub enable_ssao: bool,
    pub enable_shadow: bool,
    pub background_color: [f32; 4],
    pub model_scale: f32,
    pub materials: Vec<VfxMaterialConfig>,
    pub light_intensity: f32,
    pub light_direction: [f32; 3],
    pub ibl_intensity: f32,
    pub ibl_angle: [f32; 3],
}

impl Default for VfxSceneConfig {
    fn default() -> Self {
        Self {
            name: "None".to_string(),
            anti_aliasing: 1,
            enable_skybox: true,
            enable_bloom: true,
            enable_ssao: true,
            enable_shadow: true,
            background_color: [1.0, 1.0, 1.0, 1.0],
            model_scale: 1.0,
            materials: Vec::new(),
            light_intensity: 100_000.0,
            light_direction: [0.0, -1.0, 0.0],
            ibl_intensity: 30_000.0,
            ibl_angle: [0.0, 0.0, 0.0],
        }
    }
}

impl VfxSceneConfig {
    pub fn parse(utf8_json: &[u8]) -> Result<Self, VfxmsError> {
        let root: serde_json::Value = serde_json::from_slice(utf8_json)
            .map_err(|e| VfxmsError::InvalidJson(e.to_string()))?;

        let mut config = Self::default();

        if let Some(name) = root.get("name").and_then(|v| v.as_str()) {
            config.name = name.to_string();
        }
        if let Some(aa) = root.get("aaType").and_then(|v| v.as_i64()) {
            config.anti_aliasing = aa as i32;
        }
        if let Some(skybox) = root.get("enableSkyBox").and_then(|v| v.as_bool()) {
            config.enable_skybox = skybox;
        }
        if let Some(bloom) = root.get("enableBloom").and_then(|v| v.as_bool()) {
            config.enable_bloom = bloom;
        }
        if let Some(ssao) = root.get("enableSSAO").and_then(|v| v.as_bool()) {
            config.enable_ssao = ssao;
        }
        if let Some(shadow) = root.get("enableShadow").and_then(|v| v.as_bool()) {
            config.enable_shadow = shadow;
        }
        if let Some(scale) = root.get("modelScale").and_then(|v| v.as_f64()) {
            config.model_scale = scale as f32;
        }

        if let Some(bg) = root.get("backgroundColor").and_then(|v| v.as_array()) {
            if bg.len() == 4 {
                config.background_color = [
                    bg[0].as_f64().unwrap_or(1.0) as f32,
                    bg[1].as_f64().unwrap_or(1.0) as f32,
                    bg[2].as_f64().unwrap_or(1.0) as f32,
                    bg[3].as_f64().unwrap_or(1.0) as f32,
                ];
            }
        }

        if let Some(light) = root.get("light") {
            if let Some(intensity) = light.get("intensity").and_then(|v| v.as_f64()) {
                config.light_intensity = intensity as f32;
            }
            if let Some(dir) = light.get("direction").and_then(|v| v.as_array()) {
                if dir.len() == 3 {
                    config.light_direction = [
                        dir[0].as_f64().unwrap_or(0.0) as f32,
                        dir[1].as_f64().unwrap_or(-1.0) as f32,
                        dir[2].as_f64().unwrap_or(0.0) as f32,
                    ];
                }
            }
            if let Some(intensity) = light.get("iblIntensity").and_then(|v| v.as_f64()) {
                config.ibl_intensity = intensity as f32;
            }
            if let Some(angle) = light.get("iblAngle").and_then(|v| v.as_array()) {
                if angle.len() == 3 {
                    config.ibl_angle = [
                        angle[0].as_f64().unwrap_or(0.0) as f32,
                        angle[1].as_f64().unwrap_or(0.0) as f32,
                        angle[2].as_f64().unwrap_or(0.0) as f32,
                    ];
                }
            }
        }

        if let Some(mats) = root
            .get("materials")
            .and_then(|m| m.get("materials"))
            .and_then(|m| m.as_array())
        {
            for mat in mats {
                let mat_name = mat
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut params = Vec::new();
                if let Some(props) = mat.get("property").and_then(|v| v.as_array()) {
                    for prop in props {
                        let prop_name = prop
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let param_type = prop.get("type").and_then(|v| v.as_i64()).unwrap_or(4);
                        let is_color = param_type == 7;
                        let val = prop.get("value").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                        let mut color = [0.0; 4];
                        if let Some(c) = prop.get("color").and_then(|v| v.as_array()) {
                            if c.len() == 4 {
                                color = [
                                    c[0].as_f64().unwrap_or(0.0) as f32,
                                    c[1].as_f64().unwrap_or(0.0) as f32,
                                    c[2].as_f64().unwrap_or(0.0) as f32,
                                    c[3].as_f64().unwrap_or(0.0) as f32,
                                ];
                            }
                        }
                        params.push(VfxMaterialParameter {
                            name: prop_name,
                            value: val,
                            color,
                            is_color,
                        });
                    }
                }
                config.materials.push(VfxMaterialConfig {
                    name: mat_name,
                    parameters: params,
                });
            }
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct VfxScene {
    pub config: VfxSceneConfig,
    pub raw_config_json: Vec<u8>,
    pub glb_bytes: Vec<u8>,
    pub ibl_bytes: Vec<u8>,
}

impl VfxScene {
    pub fn parse(bytes: &[u8]) -> Result<Self, VfxmsError> {
        if bytes.len() < HEADER_SIZE {
            return Err(VfxmsError::TooShort);
        }

        let mut header = [0i32; 10];
        for (i, slot) in header.iter_mut().enumerate() {
            let chunk: [u8; 4] = bytes[i * 4..(i + 1) * 4]
                .try_into()
                .map_err(|_| VfxmsError::TooShort)?;
            *slot = i32::from_be_bytes(chunk);
        }

        let mut position = HEADER_SIZE;
        let mut next_section = |index: usize| -> Result<&[u8], VfxmsError> {
            let start = header[index * 2] as usize;
            let length = header[index * 2 + 1] as usize;
            if length == 0 {
                return Ok(&[]);
            }
            if length > bytes.len().saturating_sub(position) {
                return Err(VfxmsError::SectionExceedsFile);
            }
            if start != 0 && start != position {
                return Err(VfxmsError::InvalidSectionLayout);
            }
            let slice = &bytes[position..position + length];
            position += length;
            Ok(slice)
        };

        let config_slice = next_section(0)?;
        let glb_slice = next_section(1)?;
        let ibl_slice = next_section(2)?;

        if config_slice.is_empty() || glb_slice.is_empty() {
            return Err(VfxmsError::MissingRequiredSection);
        }

        if glb_slice.len() < 4 || &glb_slice[0..4] != GLB_MAGIC {
            return Err(VfxmsError::InvalidGlbMagic);
        }

        let config = VfxSceneConfig::parse(config_slice)?;

        Ok(Self {
            config,
            raw_config_json: config_slice.to_vec(),
            glb_bytes: glb_slice.to_vec(),
            ibl_bytes: ibl_slice.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_vfxms(config_json: &[u8], glb_data: &[u8], ibl_data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut pos = HEADER_SIZE;

        let config_start = pos as i32;
        let config_len = config_json.len() as i32;
        pos += config_json.len();

        let glb_start = pos as i32;
        let glb_len = glb_data.len() as i32;
        pos += glb_data.len();

        let ibl_start = pos as i32;
        let ibl_len = ibl_data.len() as i32;

        let header = [
            config_start,
            config_len,
            glb_start,
            glb_len,
            ibl_start,
            ibl_len,
            0,
            0,
            0,
            0,
        ];
        for val in header {
            bytes.extend_from_slice(&val.to_be_bytes());
        }
        bytes.extend_from_slice(config_json);
        bytes.extend_from_slice(glb_data);
        bytes.extend_from_slice(ibl_data);
        bytes
    }

    #[test]
    fn parses_valid_vfxms() {
        let config_json = br#"{"name":"TestScene","aaType":2,"modelScale":0.85}"#;
        let glb = b"glTF_fake_binary_model_content";
        let ibl = b"KTX_fake_ibl";

        let data = build_vfxms(config_json, glb, ibl);
        let scene = VfxScene::parse(&data).expect("should parse valid vfxms");

        assert_eq!(scene.config.name, "TestScene");
        assert_eq!(scene.config.anti_aliasing, 2);
        assert!((scene.config.model_scale - 0.85).abs() < 1e-4);
        assert_eq!(scene.glb_bytes, glb);
        assert_eq!(scene.ibl_bytes, ibl);
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(VfxScene::parse(&[0; 39]), Err(VfxmsError::TooShort));
    }

    #[test]
    fn rejects_invalid_glb_magic() {
        let config_json = br#"{"name":"Test"}"#;
        let not_glb = b"NOT_GLTF_MAGIC_DATA";
        let data = build_vfxms(config_json, not_glb, &[]);
        assert_eq!(VfxScene::parse(&data), Err(VfxmsError::InvalidGlbMagic));
    }

    #[test]
    fn rejects_truncated_section() {
        let config_json = br#"{"name":"Test"}"#;
        let glb = b"glTF_binary";
        let mut data = build_vfxms(config_json, glb, &[]);
        data.pop(); // truncate one byte
        assert_eq!(VfxScene::parse(&data), Err(VfxmsError::SectionExceedsFile));
    }
}
