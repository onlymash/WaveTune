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

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk;
use gtk::gdk::prelude::*;
use gtk::glib;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WebpFrameInfo {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u32,
    pub has_alpha: bool,
    pub payload_offset: usize,
    pub payload_len: usize,
}

#[allow(dead_code)]
pub struct AnimatedWebp {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub frames: Vec<WebpFrameInfo>,
    data: Vec<u8>,
}

impl AnimatedWebp {
    pub fn parse(bytes: Vec<u8>) -> Option<Self> {
        if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
            return None;
        }

        let mut canvas_width = 0u32;
        let mut canvas_height = 0u32;
        let mut frames = Vec::new();
        let mut pos = 12usize;

        while pos + 8 <= bytes.len() {
            let tag = &bytes[pos..pos + 4];
            let size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;

            if pos + 8 + size > bytes.len() {
                break;
            }

            let body_offset = pos + 8;
            let body = &bytes[body_offset..body_offset + size];

            if tag == b"VP8X" && body.len() >= 10 {
                canvas_width = 1 + Self::read_u24(&body[4..7]);
                canvas_height = 1 + Self::read_u24(&body[7..10]);
            } else if tag == b"ANMF" && body.len() > 16 {
                let x = Self::read_u24(&body[0..3]) * 2;
                let y = Self::read_u24(&body[3..6]) * 2;
                let width = 1 + Self::read_u24(&body[6..9]);
                let height = 1 + Self::read_u24(&body[9..12]);
                let duration_ms = Self::read_u24(&body[12..15]);
                let payload_offset = body_offset + 16;
                let payload_len = size.saturating_sub(16);
                let payload = &bytes[payload_offset..payload_offset + payload_len];
                let has_alpha = Self::contains_chunk(payload, b"ALPH");

                frames.push(WebpFrameInfo {
                    x,
                    y,
                    width,
                    height,
                    duration_ms,
                    has_alpha,
                    payload_offset,
                    payload_len,
                });
            }

            pos += 8 + size + (size & 1);
        }

        if frames.is_empty() {
            return None;
        }

        Some(Self {
            canvas_width,
            canvas_height,
            frames,
            data: bytes,
        })
    }

    pub fn to_still_webp(&self, index: usize) -> Option<Vec<u8>> {
        let frame = self.frames.get(index)?;
        let payload = &self.data[frame.payload_offset..frame.payload_offset + frame.payload_len];

        let length = 4 + 18 + payload.len();
        let mut file = Vec::with_capacity(8 + length);
        file.extend_from_slice(b"RIFF");
        file.extend_from_slice(&(length as u32).to_le_bytes());
        file.extend_from_slice(b"WEBP");
        file.extend_from_slice(b"VP8X");
        file.extend_from_slice(&10u32.to_le_bytes());

        let flags: u8 = if frame.has_alpha { 0x10 } else { 0x00 };
        file.extend_from_slice(&[flags, 0, 0, 0]);
        file.extend_from_slice(&Self::write_u24(frame.width.saturating_sub(1)));
        file.extend_from_slice(&Self::write_u24(frame.height.saturating_sub(1)));
        file.extend_from_slice(payload);

        Some(file)
    }

    fn read_u24(b: &[u8]) -> u32 {
        (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16)
    }

    fn write_u24(val: u32) -> [u8; 3] {
        [
            (val & 0xff) as u8,
            ((val >> 8) & 0xff) as u8,
            ((val >> 16) & 0xff) as u8,
        ]
    }

    fn contains_chunk(chunks: &[u8], tag: &[u8; 4]) -> bool {
        let mut pos = 0;
        while pos + 8 <= chunks.len() {
            if &chunks[pos..pos + 4] == tag {
                return true;
            }
            let size = u32::from_le_bytes([
                chunks[pos + 4],
                chunks[pos + 5],
                chunks[pos + 6],
                chunks[pos + 7],
            ]) as usize;
            pos += 8 + size + (size & 1);
        }
        false
    }
}

pub struct TurntableModel {
    webp: AnimatedWebp,
    cache: RefCell<Vec<Option<gdk::Texture>>>,
}

impl TurntableModel {
    pub fn new(webp: AnimatedWebp) -> Rc<Self> {
        let count = webp.frames.len();
        Rc::new(Self {
            webp,
            cache: RefCell::new(vec![None; count]),
        })
    }

    pub fn frame_count(&self) -> usize {
        self.webp.frames.len()
    }

    pub fn frame_duration_ms(&self, index: usize) -> u32 {
        let count = self.webp.frames.len();
        if count == 0 {
            return 33;
        }
        let dur = self.webp.frames[index % count].duration_ms;
        if dur == 0 { 33 } else { dur }
    }

    pub fn get_texture(&self, index: usize) -> Option<gdk::Texture> {
        let count = self.frame_count();
        if count == 0 {
            return None;
        }
        let index = index % count;
        if let Some(tex) = &self.cache.borrow()[index] {
            return Some(tex.clone());
        }

        let frame = self.webp.frames.get(index)?;
        let still_bytes = self.webp.to_still_webp(index)?;
        let glib_bytes = glib::Bytes::from_owned(still_bytes);
        let sub_tex = gdk::Texture::from_bytes(&glib_bytes).ok()?;

        let canvas_w = self.webp.canvas_width;
        let canvas_h = self.webp.canvas_height;

        let final_tex = if canvas_w > 0
            && canvas_h > 0
            && (canvas_w != frame.width || canvas_h != frame.height || frame.x != 0 || frame.y != 0)
        {
            let sub_w = sub_tex.width() as usize;
            let sub_h = sub_tex.height() as usize;
            let sub_stride = sub_w * 4;
            let mut sub_pixels = vec![0u8; sub_stride * sub_h];
            sub_tex.download(&mut sub_pixels, sub_stride);

            let cw = canvas_w as usize;
            let ch = canvas_h as usize;
            let canvas_stride = cw * 4;
            let mut canvas_pixels = vec![0u8; canvas_stride * ch];

            let x_offset = (frame.x as usize).min(cw);
            let y_offset = (frame.y as usize).min(ch);
            let copy_w = sub_w.min(cw.saturating_sub(x_offset));
            let copy_h = sub_h.min(ch.saturating_sub(y_offset));

            for row in 0..copy_h {
                let src_start = row * sub_stride;
                let src_end = src_start + copy_w * 4;
                let dst_start = (y_offset + row) * canvas_stride + x_offset * 4;
                let dst_end = dst_start + copy_w * 4;
                canvas_pixels[dst_start..dst_end].copy_from_slice(&sub_pixels[src_start..src_end]);
            }

            let canvas_bytes = glib::Bytes::from_owned(canvas_pixels);
            let mem_tex = gdk::MemoryTexture::new(
                canvas_w as i32,
                canvas_h as i32,
                gdk::MemoryFormat::R8g8b8a8Premultiplied,
                &canvas_bytes,
                canvas_stride,
            );
            mem_tex.upcast::<gdk::Texture>()
        } else {
            sub_tex
        };

        self.cache.borrow_mut()[index] = Some(final_tex.clone());
        Some(final_tex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_invalid_bytes_gracefully() {
        let _ = gdk::MemoryFormat::R8g8b8a8Premultiplied;
        assert!(AnimatedWebp::parse(vec![]).is_none());
        assert!(AnimatedWebp::parse(b"NOT_A_WEBP_FILE".to_vec()).is_none());
        assert!(AnimatedWebp::parse(b"RIFF\x00\x00\x00\x00WEBP".to_vec()).is_none());
    }

    #[test]
    fn parses_cached_model_if_present() {
        let cache_file = crate::resources::cache_dir().join("06EC10_model.webp");
        if cache_file.exists() {
            let bytes = std::fs::read(&cache_file).expect("read cached model");
            let model = AnimatedWebp::parse(bytes).expect("parse valid webp");
            assert_eq!(model.frames.len(), 180);
            assert_eq!(model.canvas_width, 780);
            assert_eq!(model.canvas_height, 780);

            // Raw sub-frame sizes differ depending on viewing angle
            assert_eq!(model.frames[0].width, 552);
            assert_eq!(model.frames[0].height, 484);

            let turntable = TurntableModel::new(model);
            let tex0 = turntable.get_texture(0).expect("frame 0 texture");
            assert_eq!(tex0.width(), 780);
            assert_eq!(tex0.height(), 780);

            let tex45 = turntable.get_texture(45).expect("frame 45 texture");
            assert_eq!(tex45.width(), 780);
            assert_eq!(tex45.height(), 780);
        }
    }
}
