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

use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub tex_coord: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MaterialData {
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<usize>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub metallic_roughness_texture: Option<usize>,
    pub normal_texture: Option<usize>,
    pub emissive_factor: [f32; 3],
    pub emissive_texture: Option<usize>,
}

impl Default for MaterialData {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            emissive_factor: [0.0, 0.0, 0.0],
            emissive_texture: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshPrimitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct GltfScene {
    pub primitives: Vec<MeshPrimitive>,
    pub materials: Vec<MaterialData>,
    pub textures: Vec<TextureData>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

impl GltfScene {
    pub fn from_glb_bytes(glb_bytes: &[u8]) -> Result<Self, String> {
        let (document, buffers, images) =
            gltf::import_slice(glb_bytes).map_err(|e| format!("Failed to parse GLB: {e}"))?;

        // 1. Process Textures / Images
        let mut textures = Vec::new();
        for image in images {
            let (width, height) = (image.width, image.height);
            let rgba = match image.format {
                gltf::image::Format::R8G8B8A8 => image.pixels,
                gltf::image::Format::R8G8B8 => {
                    let mut converted = Vec::with_capacity((width * height * 4) as usize);
                    for chunk in image.pixels.chunks_exact(3) {
                        converted.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                    }
                    converted
                }
                gltf::image::Format::R8 => {
                    let mut converted = Vec::with_capacity((width * height * 4) as usize);
                    for &p in &image.pixels {
                        converted.extend_from_slice(&[p, p, p, 255]);
                    }
                    converted
                }
                _ => {
                    // Fallback to placeholder 1x1 white
                    vec![255, 255, 255, 255]
                }
            };
            textures.push(TextureData {
                width,
                height,
                rgba,
            });
        }

        // 2. Process Materials
        let mut materials = Vec::new();
        for mat in document.materials() {
            let pbr = mat.pbr_metallic_roughness();
            let base_color_factor = pbr.base_color_factor();
            let base_color_texture = pbr
                .base_color_texture()
                .map(|tex| tex.texture().source().index());
            let metallic_factor = pbr.metallic_factor();
            let roughness_factor = pbr.roughness_factor();
            let metallic_roughness_texture = pbr
                .metallic_roughness_texture()
                .map(|tex| tex.texture().source().index());
            let normal_texture = mat
                .normal_texture()
                .map(|tex| tex.texture().source().index());
            let emissive_factor = mat.emissive_factor();
            let emissive_texture = mat
                .emissive_texture()
                .map(|tex| tex.texture().source().index());

            materials.push(MaterialData {
                base_color_factor,
                base_color_texture,
                metallic_factor,
                roughness_factor,
                metallic_roughness_texture,
                normal_texture,
                emissive_factor,
                emissive_texture,
            });
        }

        // 3. Process Scene Nodes and Primitives
        let mut primitives = Vec::new();
        let mut min_bound = Vec3::splat(f32::MAX);
        let mut max_bound = Vec3::splat(f32::MIN);

        for scene in document.scenes() {
            for node in scene.nodes() {
                Self::process_node(
                    &node,
                    Mat4::IDENTITY,
                    &buffers,
                    &mut primitives,
                    &mut min_bound,
                    &mut max_bound,
                );
            }
        }

        if min_bound.x > max_bound.x {
            min_bound = Vec3::splat(-1.0);
            max_bound = Vec3::splat(1.0);
        }

        Ok(Self {
            primitives,
            materials,
            textures,
            bounds_min: min_bound,
            bounds_max: max_bound,
        })
    }

    fn process_node(
        node: &gltf::Node,
        parent_transform: Mat4,
        buffers: &[gltf::buffer::Data],
        out_primitives: &mut Vec<MeshPrimitive>,
        min_bound: &mut Vec3,
        max_bound: &mut Vec3,
    ) {
        let node_matrix = Mat4::from_cols_array_2d(&node.transform().matrix());
        let current_transform = parent_transform * node_matrix;
        let normal_matrix = current_transform.inverse().transpose();

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions: Vec<[f32; 3]> = match reader.read_positions() {
                    Some(iter) => iter.collect(),
                    None => continue,
                };

                let normals: Vec<[f32; 3]> = match reader.read_normals() {
                    Some(iter) => iter.collect(),
                    None => vec![[0.0, 1.0, 0.0]; positions.len()],
                };

                let tangents: Vec<[f32; 4]> = match reader.read_tangents() {
                    Some(iter) => iter.collect(),
                    None => vec![[1.0, 0.0, 0.0, 1.0]; positions.len()],
                };

                let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                    Some(iter) => iter.into_f32().collect(),
                    None => vec![[0.0, 0.0]; positions.len()],
                };

                let indices: Vec<u32> = match reader.read_indices() {
                    Some(iter) => iter.into_u32().collect(),
                    None => (0..positions.len() as u32).collect(),
                };

                let mut vertices = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    let local_pos = Vec3::from_array(positions[i]);
                    let world_pos = current_transform.transform_point3(local_pos);

                    *min_bound = min_bound.min(world_pos);
                    *max_bound = max_bound.max(world_pos);

                    let local_norm = Vec3::from_array(normals[i]);
                    let world_norm = normal_matrix
                        .transform_vector3(local_norm)
                        .normalize_or_zero();

                    let local_tan = Vec4::from_array(tangents[i]);
                    let world_tan = normal_matrix
                        .transform_vector3(local_tan.truncate())
                        .normalize_or_zero();

                    vertices.push(Vertex {
                        position: world_pos.to_array(),
                        normal: world_norm.to_array(),
                        tangent: [world_tan.x, world_tan.y, world_tan.z, local_tan.w],
                        tex_coord: uvs[i],
                    });
                }

                out_primitives.push(MeshPrimitive {
                    vertices,
                    indices,
                    material_index: primitive.material().index(),
                });
            }
        }

        for child in node.children() {
            Self::process_node(
                &child,
                current_transform,
                buffers,
                out_primitives,
                min_bound,
                max_bound,
            );
        }
    }
}
