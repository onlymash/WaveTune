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

use std::ffi::CString;
use std::sync::OnceLock;

use glam::{Mat4, Quat, Vec3, Vec4};
use glow::HasContext;

use super::camera::OrbitCamera;
use super::ktx::{KtxCubemap, KtxPixelFormat};
use super::model::GltfScene;
use crate::resources::vfxms::VfxSceneConfig;

pub unsafe fn load_gl_func(name: &str) -> *const std::ffi::c_void {
    static EGL_PROC: OnceLock<Option<extern "C" fn(*const libc::c_char) -> *mut libc::c_void>> =
        OnceLock::new();
    static GLX_PROC: OnceLock<Option<extern "C" fn(*const libc::c_char) -> *mut libc::c_void>> =
        OnceLock::new();

    let egl = EGL_PROC.get_or_init(|| unsafe {
        let handle = libc::dlopen(c"libEGL.so.1".as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
        if !handle.is_null() {
            let sym = libc::dlsym(handle, c"eglGetProcAddress".as_ptr());
            if !sym.is_null() {
                return Some(std::mem::transmute(sym));
            }
        }
        None
    });

    let glx = GLX_PROC.get_or_init(|| unsafe {
        let handle = libc::dlopen(c"libGL.so.1".as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
        if !handle.is_null() {
            let sym = libc::dlsym(handle, c"glXGetProcAddress".as_ptr());
            if !sym.is_null() {
                return Some(std::mem::transmute(sym));
            }
        }
        None
    });

    let c_str = match CString::new(name) {
        Ok(s) => s,
        Err(_) => return std::ptr::null(),
    };

    if let Some(get_proc) = egl {
        let p = get_proc(c_str.as_ptr());
        if !p.is_null() {
            return p as *const _;
        }
    }

    if let Some(get_proc) = glx {
        let p = get_proc(c_str.as_ptr());
        if !p.is_null() {
            return p as *const _;
        }
    }

    unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_str.as_ptr()) as *const _ }
}

const VS_SOURCE: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec4 a_tangent;
layout(location = 3) in vec2 a_tex_coord;

uniform mat4 u_world;
uniform mat4 u_view_proj;

out vec3 v_world_pos;
out vec3 v_normal;
out vec4 v_tangent;
out vec2 v_tex_coord;

void main() {
    vec4 world_pos = u_world * vec4(a_position, 1.0);
    v_world_pos = world_pos.xyz;
    v_normal = mat3(u_world) * a_normal;
    v_tangent = vec4(mat3(u_world) * a_tangent.xyz, a_tangent.w);
    v_tex_coord = a_tex_coord;
    gl_Position = u_view_proj * world_pos;
}
"#;

const FS_SOURCE: &str = r#"#version 300 es
precision highp float;

in vec3 v_world_pos;
in vec3 v_normal;
in vec4 v_tangent;
in vec2 v_tex_coord;

uniform vec3 u_camera_pos;
uniform mat4 u_ibl_rotation;

// Directional light
uniform vec3 u_light_dir;
uniform vec3 u_light_color;
uniform float u_light_intensity;

// IBL parameters
uniform vec3 u_sh[9];
uniform float u_ibl_intensity;
uniform float u_max_lod;
uniform int u_has_ibl;
uniform samplerCube u_env_map;

// Material
uniform vec4 u_base_color_factor;
uniform int u_has_base_color_tex;
uniform sampler2D u_base_color_tex;

uniform float u_metallic_factor;
uniform float u_roughness_factor;
uniform int u_has_metallic_roughness_tex;
uniform sampler2D u_metallic_roughness_tex;

uniform int u_has_normal_tex;
uniform sampler2D u_normal_tex;

uniform vec3 u_emissive_factor;
uniform int u_has_emissive_tex;
uniform sampler2D u_emissive_tex;

out vec4 frag_color;

const float PI = 3.14159265359;

vec3 irradiance(vec3 n) {
    return max(u_sh[0]
        + u_sh[1] * n.y + u_sh[2] * n.z + u_sh[3] * n.x
        + u_sh[4] * (n.y * n.x) + u_sh[5] * (n.y * n.z) + u_sh[6] * (3.0 * n.z * n.z - 1.0)
        + u_sh[7] * (n.z * n.x) + u_sh[8] * (n.x * n.x - n.y * n.y), vec3(0.0));
}

vec2 dfg(float NoV, float perceptual_roughness) {
    vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    vec4 c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    vec4 r = perceptual_roughness * c0 + c1;
    float a004 = min(r.x * r.x, exp2(-9.28 * NoV)) * r.x + r.y;
    return vec2(-1.04, 1.04) * a004 + r.zw;
}

float specular_ao(float NoV, float ao, float roughness) {
    return clamp(pow(NoV + ao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ao, 0.0, 1.0);
}

float d_ggx(float NoH, float roughness) {
    float a = NoH * roughness;
    float k = roughness / (1.0 - NoH * NoH + a * a);
    return k * k * (1.0 / PI);
}

float v_smith_ggx_correlated(float NoV, float NoL, float roughness) {
    float a2 = roughness * roughness;
    float lv = NoL * sqrt(NoV * NoV * (1.0 - a2) + a2);
    float ll = NoV * sqrt(NoL * NoL * (1.0 - a2) + a2);
    return 0.5 / (lv + ll);
}

vec3 f_schlick(vec3 f0, float VoH) {
    float f = pow(1.0 - VoH, 5.0);
    return f + f0 * (1.0 - f);
}

// Filament ACES_LEGACY tone mapping
vec3 tone_map(vec3 x) {
    mat3 input_mat = mat3(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777
    );
    mat3 output_mat = mat3(
        1.60475, -0.10208, -0.00327,
        -0.53108, 1.10813, -0.07276,
        -0.07367, -0.00605, 1.07602
    );
    vec3 v = input_mat * (x / 0.6);
    vec3 a = v * (v + 0.0245786) - 0.000090537;
    vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return clamp(output_mat * (a / b), 0.0, 1.0);
}

vec3 linear_to_srgb(vec3 c) {
    return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, c));
}

void main() {
    vec4 base_color = u_base_color_factor;
    if (u_has_base_color_tex == 1) {
        vec4 tex_col = texture(u_base_color_tex, v_tex_coord);
        // Base color textures in glTF are sRGB; convert to linear space for physically based lighting:
        tex_col.rgb = pow(tex_col.rgb, vec3(2.2));
        base_color *= tex_col;
    }

    if (base_color.a < 0.05) {
        discard;
    }

    float metallic = clamp(u_metallic_factor, 0.0, 1.0);
    float perceptual_roughness = clamp(u_roughness_factor, 0.045, 1.0);
    if (u_has_metallic_roughness_tex == 1) {
        vec4 mr = texture(u_metallic_roughness_tex, v_tex_coord);
        perceptual_roughness *= mr.g;
        metallic *= mr.b;
    }
    float specular_roughness = perceptual_roughness;
    perceptual_roughness = clamp(perceptual_roughness, 0.045, 1.0);
    float roughness = perceptual_roughness * perceptual_roughness;

    vec3 N = normalize(v_normal);
    if (u_has_normal_tex == 1 && length(v_tangent.xyz) > 0.01) {
        vec3 T = normalize(v_tangent.xyz - dot(v_tangent.xyz, N) * N);
        vec3 B = cross(N, T) * sign(v_tangent.w);
        mat3 TBN = mat3(T, B, N);
        vec3 normal_sample = texture(u_normal_tex, v_tex_coord).rgb * 2.0 - 1.0;
        N = normalize(TBN * normal_sample);
    }

    vec3 V = normalize(u_camera_pos - v_world_pos);
    float NoV = max(dot(N, V), 1e-4);

    float reflectance = 0.5;
    vec3 diffuse_color = base_color.rgb * (1.0 - metallic);
    vec3 f0 = 0.16 * reflectance * reflectance * (1.0 - metallic) + base_color.rgb * metallic;

    vec3 color = vec3(0.0);

    // 1. IBL Lighting (Indirect Environment Light)
    if (u_has_ibl == 1) {
        vec2 env_dfg = dfg(NoV, perceptual_roughness);
        vec3 E = f0 * env_dfg.x + env_dfg.y;
        vec3 r = reflect(-V, N);
        r = mix(r, N, roughness * roughness);

        vec3 env_r = mat3(u_ibl_rotation) * r;
        vec3 env_n = mat3(u_ibl_rotation) * N;

        float lod = clamp(u_max_lod * specular_roughness * (2.0 - specular_roughness), 0.0, u_max_lod);
        vec3 radiance = textureLod(u_env_map, env_r, lod).rgb;
        // Floor environment radiance with diffuse ambient irradiance so metals retain clean metallic shine
        radiance = max(radiance, irradiance(env_r) * 0.45);

        vec3 Fr = E * radiance * specular_ao(NoV, 1.0, roughness);
        vec3 Fd = diffuse_color * irradiance(env_n) * (vec3(1.0) - E);
        color += (Fd + Fr) * u_ibl_intensity;
    } else {
        // Fallback lighting when no IBL cubemap is supplied: studio ambient + key
        vec3 env_n = mat3(u_ibl_rotation) * N;
        vec3 Fd = diffuse_color * irradiance(env_n);
        color += Fd * max(u_ibl_intensity, 1.2);
    }

    // 2. Directional Light (if enabled in scene)
    if (u_light_intensity > 0.0) {
        vec3 L = normalize(-u_light_dir);
        float NoL = clamp(dot(N, L), 0.0, 1.0);
        if (NoL > 0.0) {
            vec3 H = normalize(V + L);
            float NoH = clamp(dot(N, H), 0.0, 1.0);
            float VoH = clamp(dot(V, H), 0.0, 1.0);
            vec3 Fr = d_ggx(NoH, roughness) * v_smith_ggx_correlated(NoV, NoL, roughness) * f_schlick(f0, VoH);
            vec3 Fd = diffuse_color / PI;
            color += (Fd + Fr) * u_light_color * (u_light_intensity * NoL);
        }
    }

    // 3. Emissive
    vec3 emissive = u_emissive_factor;
    if (u_has_emissive_tex == 1) {
        emissive *= texture(u_emissive_tex, v_tex_coord).rgb;
    }
    color += emissive;

    // 4. Tone Mapping & sRGB Conversion
    vec3 tone_mapped = tone_map(color);
    vec3 srgb = linear_to_srgb(tone_mapped);

    frag_color = vec4(srgb * base_color.a, base_color.a);
}
"#;

struct GpuPrimitive {
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
    index_count: i32,
    material_index: Option<usize>,
}

pub struct SceneRenderer {
    gl: glow::Context,
    program: glow::Program,
    gpu_primitives: Vec<GpuPrimitive>,
    gpu_textures: Vec<glow::Texture>,
    env_cubemap: Option<glow::Texture>,
    sh_coefficients: [Vec3; 9],
    max_lod: f32,
    ibl_rotation: Mat4,
    scene: GltfScene,
    config: VfxSceneConfig,
}

impl SceneRenderer {
    pub fn new(scene: GltfScene, config: VfxSceneConfig, ibl_bytes: &[u8]) -> Result<Self, String> {
        let gl = unsafe { glow::Context::from_loader_function(|s| load_gl_func(s)) };

        unsafe {
            let vs = gl
                .create_shader(glow::VERTEX_SHADER)
                .map_err(|e| format!("VS create error: {e}"))?;
            gl.shader_source(vs, VS_SOURCE);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                let info = gl.get_shader_info_log(vs);
                return Err(format!("VS compile failed: {info}"));
            }

            let fs = gl
                .create_shader(glow::FRAGMENT_SHADER)
                .map_err(|e| format!("FS create error: {e}"))?;
            gl.shader_source(fs, FS_SOURCE);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                let info = gl.get_shader_info_log(fs);
                return Err(format!("FS compile failed: {info}"));
            }

            let program = gl
                .create_program()
                .map_err(|e| format!("Program create error: {e}"))?;
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let info = gl.get_program_info_log(program);
                return Err(format!("Program link failed: {info}"));
            }
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            // Upload Material Textures
            let mut gpu_textures = Vec::new();
            for tex_data in &scene.textures {
                let tex = gl
                    .create_texture()
                    .map_err(|e| format!("Texture create error: {e}"))?;
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    tex_data.width as i32,
                    tex_data.height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&tex_data.rgba)),
                );
                gl.generate_mipmap(glow::TEXTURE_2D);
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR_MIPMAP_LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
                gpu_textures.push(tex);
            }

            // Upload Environment Cubemap from IBL section
            let mut env_cubemap = None;
            let mut sh_coefficients = [Vec3::ZERO; 9];
            // Default ambient SH if no IBL exists
            sh_coefficients[0] = Vec3::splat(1.2);
            sh_coefficients[1] = Vec3::new(0.3, 0.5, 0.3);
            let mut max_lod = 0.0;

            if !ibl_bytes.is_empty() {
                if let Ok(cubemap) = KtxCubemap::parse(ibl_bytes) {
                    if let Some(sh) = cubemap.spherical_harmonics {
                        sh_coefficients = sh;
                    }
                    max_lod = (cubemap.levels.len().saturating_sub(1)) as f32;

                    if let Ok(cube_tex) = gl.create_texture() {
                        gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(cube_tex));

                        let (internal_format, gl_format, gl_type) = match cubemap.format {
                            KtxPixelFormat::R11G11B10Float => (
                                glow::R11F_G11F_B10F,
                                glow::RGB,
                                glow::UNSIGNED_INT_10F_11F_11F_REV,
                            ),
                            KtxPixelFormat::Rgba16Float => {
                                (glow::RGBA16F, glow::RGBA, glow::HALF_FLOAT)
                            }
                            KtxPixelFormat::Rgba8 => (glow::RGBA8, glow::RGBA, glow::UNSIGNED_BYTE),
                        };

                        for (level, faces) in cubemap.levels.iter().enumerate() {
                            let edge = (cubemap.size >> level).max(1);
                            for (face_idx, face_bytes) in faces.iter().enumerate() {
                                gl.tex_image_2d(
                                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + face_idx as u32,
                                    level as i32,
                                    internal_format as i32,
                                    edge as i32,
                                    edge as i32,
                                    0,
                                    gl_format,
                                    gl_type,
                                    glow::PixelUnpackData::Slice(Some(face_bytes)),
                                );
                            }
                        }

                        gl.tex_parameter_i32(glow::TEXTURE_CUBE_MAP, glow::TEXTURE_BASE_LEVEL, 0);
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_MAX_LEVEL,
                            (cubemap.levels.len().saturating_sub(1)) as i32,
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_MIN_FILTER,
                            if cubemap.levels.len() > 1 {
                                glow::LINEAR_MIPMAP_LINEAR as i32
                            } else {
                                glow::LINEAR as i32
                            },
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_MAG_FILTER,
                            glow::LINEAR as i32,
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_WRAP_S,
                            glow::CLAMP_TO_EDGE as i32,
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_WRAP_T,
                            glow::CLAMP_TO_EDGE as i32,
                        );
                        gl.tex_parameter_i32(
                            glow::TEXTURE_CUBE_MAP,
                            glow::TEXTURE_WRAP_R,
                            glow::CLAMP_TO_EDGE as i32,
                        );

                        env_cubemap = Some(cube_tex);
                    }
                }
            }

            // Calculate IBL rotation matrix from config.ibl_angle (Euler order: Z then X then Y)
            let angle = config.ibl_angle;
            let rot_z = Quat::from_axis_angle(Vec3::Z, angle[2].to_radians());
            let rot_x = Quat::from_axis_angle(Vec3::X, angle[0].to_radians());
            let rot_y = Quat::from_axis_angle(Vec3::Y, angle[1].to_radians());
            let rotation = rot_y * rot_x * rot_z;
            let ibl_rotation = Mat4::from_quat(rotation).transpose();

            // Upload Primitives
            let mut gpu_primitives = Vec::new();
            for prim in &scene.primitives {
                let vao = gl
                    .create_vertex_array()
                    .map_err(|e| format!("VAO create error: {e}"))?;
                gl.bind_vertex_array(Some(vao));

                let vbo = gl
                    .create_buffer()
                    .map_err(|e| format!("VBO create error: {e}"))?;
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                let vertex_bytes = bytemuck::cast_slice(&prim.vertices);
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertex_bytes, glow::STATIC_DRAW);

                let ebo = gl
                    .create_buffer()
                    .map_err(|e| format!("EBO create error: {e}"))?;
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
                let index_bytes = bytemuck::cast_slice(&prim.indices);
                gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, index_bytes, glow::STATIC_DRAW);

                let stride = std::mem::size_of::<super::model::Vertex>() as i32;

                // 0: Position
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

                // 1: Normal
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

                // 2: Tangent
                gl.enable_vertex_attrib_array(2);
                gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);

                // 3: TexCoord
                gl.enable_vertex_attrib_array(3);
                gl.vertex_attrib_pointer_f32(3, 2, glow::FLOAT, false, stride, 40);

                gl.bind_vertex_array(None);

                gpu_primitives.push(GpuPrimitive {
                    vao,
                    _vbo: vbo,
                    _ebo: ebo,
                    index_count: prim.indices.len() as i32,
                    material_index: prim.material_index,
                });
            }

            Ok(Self {
                gl,
                program,
                gpu_primitives,
                gpu_textures,
                env_cubemap,
                sh_coefficients,
                max_lod,
                ibl_rotation,
                scene,
                config,
            })
        }
    }

    pub fn render(&self, camera: &OrbitCamera, width: i32, height: i32) {
        unsafe {
            self.gl.viewport(0, 0, width, height);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LEQUAL);
            self.gl.enable(glow::BLEND);
            self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);

            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.gl.use_program(Some(self.program));

            let aspect = width as f32 / height.max(1) as f32;
            let view = camera.view_matrix();
            let proj = camera.projection_matrix(aspect);
            let view_proj = proj * view;

            let world = OrbitCamera::compute_model_matrix(
                self.scene.bounds_min,
                self.scene.bounds_max,
                self.config.model_scale,
            );

            self.set_mat4("u_world", &world);
            self.set_mat4("u_view_proj", &view_proj);
            self.set_mat4("u_ibl_rotation", &self.ibl_rotation);

            let cam_pos = camera.eye_position();
            self.set_vec3("u_camera_pos", cam_pos);

            // Filament standard camera exposure factor: f/16, 1/125s, ISO 100
            let exposure = 1.0 / (1.2 * (16.0 * 16.0 / 0.008));

            // Directional light (from config)
            let light_dir = Vec3::from_array(self.config.light_direction);
            self.set_vec3("u_light_dir", light_dir);
            self.set_vec3("u_light_color", Vec3::new(1.0, 0.973, 0.996));
            self.set_float(
                "u_light_intensity",
                self.config.light_intensity.max(0.0) * exposure,
            );

            // IBL uniforms
            self.set_vec3_array("u_sh", &self.sh_coefficients);
            self.set_float("u_ibl_intensity", self.config.ibl_intensity * exposure);
            self.set_float("u_max_lod", self.max_lod);

            if let Some(env_tex) = self.env_cubemap {
                self.gl.active_texture(glow::TEXTURE4);
                self.gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(env_tex));
                self.set_int("u_env_map", 4);
                self.set_int("u_has_ibl", 1);
            } else {
                self.set_int("u_has_ibl", 0);
            }

            for prim in &self.gpu_primitives {
                let default_mat = super::model::MaterialData::default();
                let mat = prim
                    .material_index
                    .and_then(|idx| self.scene.materials.get(idx))
                    .unwrap_or(&default_mat);

                self.set_vec4(
                    "u_base_color_factor",
                    Vec4::from_array(mat.base_color_factor),
                );
                self.set_float("u_metallic_factor", mat.metallic_factor);
                self.set_float("u_roughness_factor", mat.roughness_factor);
                self.set_vec3("u_emissive_factor", Vec3::from_array(mat.emissive_factor));

                // Bind Textures
                self.bind_texture_uniform(
                    0,
                    mat.base_color_texture,
                    "u_base_color_tex",
                    "u_has_base_color_tex",
                );
                self.bind_texture_uniform(
                    1,
                    mat.metallic_roughness_texture,
                    "u_metallic_roughness_tex",
                    "u_has_metallic_roughness_tex",
                );
                self.bind_texture_uniform(
                    2,
                    mat.normal_texture,
                    "u_normal_tex",
                    "u_has_normal_tex",
                );
                self.bind_texture_uniform(
                    3,
                    mat.emissive_texture,
                    "u_emissive_tex",
                    "u_has_emissive_tex",
                );

                self.gl.bind_vertex_array(Some(prim.vao));
                self.gl
                    .draw_elements(glow::TRIANGLES, prim.index_count, glow::UNSIGNED_INT, 0);
            }

            self.gl.bind_vertex_array(None);
            self.gl.use_program(None);
        }
    }

    unsafe fn bind_texture_uniform(
        &self,
        slot: u32,
        tex_index: Option<usize>,
        tex_uniform: &str,
        has_uniform: &str,
    ) {
        unsafe {
            if let Some(idx) = tex_index {
                if let Some(tex) = self.gpu_textures.get(idx) {
                    self.gl.active_texture(glow::TEXTURE0 + slot);
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                    self.set_int(tex_uniform, slot as i32);
                    self.set_int(has_uniform, 1);
                    return;
                }
            }
            self.set_int(has_uniform, 0);
        }
    }

    unsafe fn set_mat4(&self, name: &str, mat: &Mat4) {
        unsafe {
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&loc), false, &mat.to_cols_array());
            }
        }
    }

    unsafe fn set_vec3(&self, name: &str, v: Vec3) {
        unsafe {
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl.uniform_3_f32(Some(&loc), v.x, v.y, v.z);
            }
        }
    }

    unsafe fn set_vec3_array(&self, name: &str, values: &[Vec3]) {
        unsafe {
            let mut flat = Vec::with_capacity(values.len() * 3);
            for v in values {
                flat.extend_from_slice(&[v.x, v.y, v.z]);
            }
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl.uniform_3_f32_slice(Some(&loc), &flat);
            }
        }
    }

    unsafe fn set_vec4(&self, name: &str, v: Vec4) {
        unsafe {
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl.uniform_4_f32(Some(&loc), v.x, v.y, v.z, v.w);
            }
        }
    }

    unsafe fn set_float(&self, name: &str, val: f32) {
        unsafe {
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl.uniform_1_f32(Some(&loc), val);
            }
        }
    }

    unsafe fn set_int(&self, name: &str, val: i32) {
        unsafe {
            if let Some(loc) = self.gl.get_uniform_location(self.program, name) {
                self.gl.uniform_1_i32(Some(&loc), val);
            }
        }
    }
}

impl Drop for SceneRenderer {
    fn drop(&mut self) {
        unsafe {
            for prim in &self.gpu_primitives {
                self.gl.delete_vertex_array(prim.vao);
            }
            for tex in &self.gpu_textures {
                self.gl.delete_texture(*tex);
            }
            if let Some(env_tex) = self.env_cubemap {
                self.gl.delete_texture(env_tex);
            }
            self.gl.delete_program(self.program);
        }
    }
}
