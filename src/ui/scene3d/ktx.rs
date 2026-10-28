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

use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KtxPixelFormat {
    R11G11B10Float,
    Rgba16Float,
    Rgba8,
}

#[derive(Debug, Clone)]
pub struct KtxCubemap {
    pub format: KtxPixelFormat,
    pub size: u32,
    pub levels: Vec<[Vec<u8>; 6]>,
    pub spherical_harmonics: Option<[Vec3; 9]>,
}

const IDENTIFIER: &[u8; 12] = &[
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

impl KtxCubemap {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 64 || &data[..12] != IDENTIFIER {
            return Err("Not a valid KTX 1.1 file".to_string());
        }

        let endianness = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let little = endianness == 0x04030201;

        let read_u32 = |offset: usize| -> u32 {
            let chunk = data[offset..offset + 4].try_into().unwrap();
            if little {
                u32::from_le_bytes(chunk)
            } else {
                u32::from_be_bytes(chunk)
            }
        };

        let gl_internal_format = read_u32(28);
        let width = read_u32(36);
        let height = read_u32(40);
        let faces = read_u32(52);
        let mips = read_u32(56).max(1);
        let kv_bytes = read_u32(60) as usize;

        if faces != 6 || width != height || width == 0 || width > 4096 || mips > 13 {
            return Err("Unsupported KTX cubemap dimensions".to_string());
        }

        let format = match gl_internal_format {
            0x8C3A => KtxPixelFormat::R11G11B10Float,
            0x881A => KtxPixelFormat::Rgba16Float,
            0x8058 | 0x8C43 => KtxPixelFormat::Rgba8,
            other => return Err(format!("Unsupported KTX format 0x{other:X}")),
        };

        if 64 + kv_bytes > data.len() {
            return Err("Corrupt KTX metadata".to_string());
        }

        let mut sh = None;
        let mut pos = 64;
        while pos + 4 <= 64 + kv_bytes {
            let size = read_u32(pos) as usize;
            if pos + 4 + size > 64 + kv_bytes {
                break;
            }
            let pair = &data[pos + 4..pos + 4 + size];
            if let Some(zero_idx) = pair.iter().position(|&b| b == 0) {
                if &pair[..zero_idx] == b"sh" {
                    if let Ok(sh_str) = std::str::from_utf8(&pair[zero_idx + 1..]) {
                        sh = Self::parse_spherical_harmonics(sh_str.trim_matches('\0'));
                    }
                }
            }
            pos += 4 + ((size + 3) & !3);
        }

        let bpp = match format {
            KtxPixelFormat::Rgba16Float => 8,
            _ => 4,
        };

        let mut levels = Vec::with_capacity(mips as usize);
        let mut offset = 64 + kv_bytes;

        for level in 0..mips {
            let edge = (width >> level).max(1) as usize;
            let face_bytes = edge * edge * bpp;
            if offset + 4 > data.len() {
                return Err("Truncated KTX file".to_string());
            }
            let image_size = read_u32(offset) as usize;
            offset += 4;
            if image_size < face_bytes {
                return Err("Unexpected KTX image size".to_string());
            }

            let mut face_data: [Vec<u8>; 6] = Default::default();
            for face in 0..6 {
                if offset + face_bytes > data.len() {
                    return Err("Truncated KTX face data".to_string());
                }
                face_data[face] = data[offset..offset + face_bytes].to_vec();
                offset += (image_size + 3) & !3;
            }
            levels.push(face_data);
        }

        Ok(Self {
            format,
            size: width,
            levels,
            spherical_harmonics: sh,
        })
    }

    fn parse_spherical_harmonics(text: &str) -> Option<[Vec3; 9]> {
        let floats: Vec<f32> = text
            .split_whitespace()
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        if floats.len() < 27 {
            return None;
        }
        let mut sh = [Vec3::ZERO; 9];
        for i in 0..9 {
            sh[i] = Vec3::new(floats[i * 3], floats[i * 3 + 1], floats[i * 3 + 2]);
        }
        Some(sh)
    }
}
