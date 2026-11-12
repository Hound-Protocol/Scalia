//! The wgpu renderer: shadow pass, scene pass into a low-resolution target (colour, normal, depth),
//! then an upscale pass that adds outlines and undoes the camera snap.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::camera::{light_matrix, snap_target_at, view_at};
use crate::shaders::{BLIT_WGSL, SCENE_WGSL, SHADOW_WGSL};
use crate::sky::Sky;

pub const SHADOW_SIZE: u32 = 2048;
/// Bytes per instance: (x, y, z, body palette), (heading, phase offset, steps per second, pose) and
/// (previous pose, time of the change in seconds, 0, 0).
pub const INSTANCE_BYTES: usize = 48;

/// Sixteen-colour palette.
pub const PALETTE: [[f32; 3]; 32] = [
    [0.10, 0.12, 0.15], [0.42, 0.46, 0.52], [0.55, 0.58, 0.60], [0.30, 0.33, 0.38],
    [0.72, 0.58, 0.20], [0.80, 0.35, 0.15], [0.25, 0.50, 0.70], [0.30, 0.62, 0.40],
    [0.90, 0.88, 0.80], [0.60, 0.20, 0.20], [0.25, 0.40, 0.20], [0.32, 0.48, 0.24],
    [0.40, 0.52, 0.28], [0.48, 0.44, 0.30], [0.95, 0.80, 0.35], [0.85, 0.85, 0.90],
    // 16-31: the basic soldier's kit (see mesh::kit): tan uniform, dark vest, boots and gloves.
    [0.62, 0.60, 0.44], [0.49, 0.47, 0.34], [0.17, 0.18, 0.19], [0.58, 0.60, 0.62],
    [0.40, 0.38, 0.28], [0.25, 0.26, 0.27], [0.12, 0.12, 0.13], [0.13, 0.14, 0.15],
    [0.20, 0.21, 0.22], [0.10, 0.11, 0.12], [0.63, 0.61, 0.45], [0.86, 0.70, 0.55],
    [0.08, 0.08, 0.09], [0.17, 0.18, 0.19], [0.72, 0.62, 0.42], [0.70, 0.72, 0.66],
];

/// What to draw in one frame.
pub struct FrameDesc<'a> {
    pub meshes: &'a [String],
    /// (mesh key, instance buffer key) pairs.
    pub instanced: &'a [(String, String)],
    /// Instanced decoration drawn in the scene but casting no shadow (grass).
    pub decor: &'a [(String, String)],
    pub target: Vec3,
    pub yaw: f32,
    pub view_height: f32,
    pub range: f32,
    pub internal: (u32, u32),
    pub outline: bool,
    pub sky: Sky,
    /// Haze and sky background (ground and ships); off for space.
    pub atmosphere: bool,
    /// Seconds for animation; keep it small (wrap it) so f32 stays precise.
    pub time_s: f32,
    /// Camera elevation above the horizon (radians); the game uses `camera::ELEV`.
    pub elev: f32,
    /// How far the flat planet map is bent into a globe (see `bend`).
    pub bend: crate::bend::Bend,
}

struct Targets {
    size: (u32, u32),
    color: wgpu::TextureView,
    normal: wgpu::TextureView,
    depth: wgpu::TextureView,
    blit_bind: wgpu::BindGroup,
}

/// GPU timestamps around the three passes, read back without stalling: a new measurement starts only
/// after the previous one has been mapped and read.
struct Timing {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    period_ns: f32,
    /// 0 idle, 1 waiting for the map, 2 mapped and ready to read.
    state: Arc<Mutex<u8>>,
    /// Milliseconds for the shadow, scene and upscale passes.
    last: Option<[f32; 3]>,
}

struct Gpubuf {
    buf: wgpu::Buffer,
    count: u32,
}

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    scene_pipe: wgpu::RenderPipeline,
    shadow_pipe: wgpu::RenderPipeline,
    blit_pipe: wgpu::RenderPipeline,
    uni: wgpu::Buffer,
    b_uni: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    shadow_bind: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    one_instance: wgpu::Buffer,
    targets: Option<Targets>,
    buffers: HashMap<String, Gpubuf>,
    /// Adapter description for the HUD.
    pub adapter_name: String,
    /// True when the adapter is a CPU (software) renderer.
    pub software: bool,
    timing: Option<Timing>,
    instance: wgpu::Instance,
    /// A second drawing surface (the tablet's map screen) with its own size and render targets.
    second: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration, Option<Targets>)>,
}

fn vertex_layouts() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    const MESH: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];
    const INST: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
    [
        Some(wgpu::VertexBufferLayout { array_stride: 16, step_mode: wgpu::VertexStepMode::Vertex, attributes: &MESH }),
        Some(wgpu::VertexBufferLayout { array_stride: INSTANCE_BYTES as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &INST }),
    ]
}

fn f32_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

impl Gpu {
    pub async fn new(target: impl Into<wgpu::SurfaceTarget<'static>>, width: u32, height: u32) -> Result<Gpu, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(target).map_err(|e| format!("surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("no WebGPU adapter: {e}"))?;
        let info = adapter.get_info();
        let software = info.device_type == wgpu::DeviceType::Cpu
            || info.name.to_lowercase().contains("swiftshader")
            || info.driver.to_lowercase().contains("swiftshader");
        let adapter_name = if info.name.is_empty() { format!("{:?}", info.backend) } else { info.name.clone() };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("device: {e}"))?;
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or("surface not supported by adapter")?;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface.configure(&device, &config);
        let format = config.format;

        let uni = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene uniforms"),
            size: 752,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let b_uni = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("upscale uniforms"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let one_instance = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("identity instance"),
            contents: f32_bytes(&[0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&Default::default());
        let cmp = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::Less),
            ..Default::default()
        });

        let scene_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(SCENE_WGSL.into()),
        });
        let shadow_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_WGSL.into()),
        });
        let blit_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("upscale"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let layouts = vertex_layouts();
        let tri = wgpu::PrimitiveState { cull_mode: None, ..Default::default() };
        let depth = |format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let rgba = Some(wgpu::ColorTargetState::from(wgpu::TextureFormat::Rgba8Unorm));
        let scene_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene"),
            layout: None,
            vertex: wgpu::VertexState { module: &scene_mod, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &layouts },
            primitive: tri,
            depth_stencil: Some(depth(wgpu::TextureFormat::Depth24Plus)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &scene_mod,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[rgba.clone(), rgba],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: None,
            vertex: wgpu::VertexState { module: &shadow_mod, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &layouts },
            primitive: tri,
            depth_stencil: Some(depth(wgpu::TextureFormat::Depth32Float)),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let blit_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("upscale"),
            layout: None,
            vertex: wgpu::VertexState { module: &blit_mod, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &blit_mod,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &scene_pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uni.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&cmp) },
            ],
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow"),
            layout: &shadow_pipe.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uni.as_entire_binding() }],
        });

        let timing = device.features().contains(wgpu::Features::TIMESTAMP_QUERY).then(|| Timing {
            set: device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("pass times"), ty: wgpu::QueryType::Timestamp, count: 6 }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamps"),
                size: 48,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamps readback"),
                size: 48,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            period_ns: queue.get_timestamp_period(),
            state: Arc::new(Mutex::new(0)),
            last: None,
        });

        Ok(Gpu {
            timing,
            device, queue, surface, config, scene_pipe, shadow_pipe, blit_pipe, uni, b_uni, scene_bind, shadow_bind,
            shadow_view, one_instance, targets: None, buffers: HashMap::new(), adapter_name, software, instance, second: None,
        })
    }

    /// Adds a second surface (another canvas) drawn with `frame_second`; it shares meshes and pipelines.
    pub fn add_second(&mut self, target: impl Into<wgpu::SurfaceTarget<'static>>, width: u32, height: u32) -> Result<(), String> {
        let surface = self.instance.create_surface(target).map_err(|e| format!("second surface: {e}"))?;
        let mut config = self.config.clone();
        config.width = width.max(1);
        config.height = height.max(1);
        surface.configure(&self.device, &config);
        self.second = Some((surface, config, None));
        Ok(())
    }

    pub fn resize_second(&mut self, width: u32, height: u32) {
        if let Some((surface, config, _)) = self.second.as_mut() {
            if width > 0 && height > 0 && (width, height) != (config.width, config.height) {
                config.width = width;
                config.height = height;
                surface.configure(&self.device, config);
            }
        }
    }

    pub fn has_second(&self) -> bool {
        self.second.is_some()
    }

    /// Draws a frame onto the second surface.
    pub fn frame_second(&mut self, f: &FrameDesc) -> Result<(), String> {
        let Some(mut sec) = self.second.take() else { return Ok(()) };
        std::mem::swap(&mut self.surface, &mut sec.0);
        std::mem::swap(&mut self.config, &mut sec.1);
        std::mem::swap(&mut self.targets, &mut sec.2);
        let r = self.frame(f);
        std::mem::swap(&mut self.surface, &mut sec.0);
        std::mem::swap(&mut self.config, &mut sec.1);
        std::mem::swap(&mut self.targets, &mut sec.2);
        self.second = Some(sec);
        r
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width, height) == (self.config.width, self.config.height) {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Stores (or replaces) mesh vertices (16-byte records).
    pub fn upload(&mut self, key: &str, bytes: &[u8]) {
        self.upload_records(key, bytes, 16);
    }

    /// Stores (or replaces) instances (`INSTANCE_BYTES` each).
    pub fn upload_instances(&mut self, key: &str, bytes: &[u8]) {
        self.upload_records(key, bytes, INSTANCE_BYTES);
    }

    fn upload_records(&mut self, key: &str, bytes: &[u8], record: usize) {
        let need = bytes.len().max(16) as u64;
        let fresh = match self.buffers.get(key) {
            Some(b) => b.buf.size() < need,
            None => true,
        };
        if fresh {
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(key),
                size: need.next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffers.insert(key.to_string(), Gpubuf { buf, count: 0 });
        }
        let b = self.buffers.get_mut(key).expect("just inserted");
        if !bytes.is_empty() {
            self.queue.write_buffer(&b.buf, 0, bytes);
        }
        b.count = (bytes.len() / record) as u32;
    }

    /// Last measured GPU milliseconds for the shadow, scene and upscale passes, when the browser allows timestamps.
    pub fn gpu_ms(&self) -> Option<[f32; 3]> {
        self.timing.as_ref().and_then(|t| t.last)
    }

    /// Reads a finished measurement, if any; returns whether a new one may be recorded this frame.
    fn poll_timing(&mut self) -> bool {
        let Some(t) = self.timing.as_mut() else { return false };
        let state = *t.state.lock().expect("timing lock");
        if state == 2 {
            {
                let view = t.readback.slice(..).get_mapped_range().expect("mapped");
                let v: Vec<u64> = view.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes"))).collect();
                let ms = |a: u64, b: u64| b.saturating_sub(a) as f32 * t.period_ns / 1e6;
                t.last = Some([ms(v[0], v[1]), ms(v[2], v[3]), ms(v[4], v[5])]);
            }
            t.readback.unmap();
            *t.state.lock().expect("timing lock") = 0;
            return true;
        }
        state == 0
    }

    pub fn has(&self, key: &str) -> bool {
        self.buffers.contains_key(key)
    }

    pub fn drop_buffer(&mut self, key: &str) {
        self.buffers.remove(key);
    }

    fn ensure_targets(&mut self, w: u32, h: u32) {
        if self.targets.as_ref().is_some_and(|t| t.size == (w, h)) {
            return;
        }
        let make = |format, label| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: w + 2, height: h + 2, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let color = make(wgpu::TextureFormat::Rgba8Unorm, "colour");
        let normal = make(wgpu::TextureFormat::Rgba8Unorm, "normal");
        let depth = make(wgpu::TextureFormat::Depth24Plus, "depth");
        let blit_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upscale"),
            layout: &self.blit_pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&color) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&normal) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&depth) },
                wgpu::BindGroupEntry { binding: 3, resource: self.b_uni.as_entire_binding() },
            ],
        });
        self.targets = Some(Targets { size: (w, h), color, normal, depth, blit_bind });
    }

    pub fn frame(&mut self, f: &FrameDesc) -> Result<(), String> {
        let (w, h) = f.internal;
        self.ensure_targets(w, h);
        let snap = snap_target_at(f.target, f.yaw, f.elev, f.view_height / h as f32);
        let (m, vdir) = view_at(snap.target, f.yaw, f.elev, f.view_height * (h + 2) as f32 / h as f32, (w + 2) as f32 / (h + 2) as f32, f.range);
        // The sun camera spans 4 view heights in depth: plenty for the scene, and fine depth precision.
        let light_range = f.view_height * 4.0;
        let lm = light_matrix(f.sky.light_dir, snap.target, f.view_height * 1.3, light_range, SHADOW_SIZE);
        let mut u = Vec::with_capacity(100);
        u.extend_from_slice(&m.to_cols_array());
        u.extend_from_slice(&lm.to_cols_array());
        u.extend_from_slice(&[vdir.x, vdir.y, vdir.z, 0.0]);
        for c in PALETTE {
            u.extend_from_slice(&[c[0], c[1], c[2], 1.0]);
        }
        let (l, lc, am) = (f.sky.light_dir, f.sky.light_col, f.sky.ambient);
        u.extend_from_slice(&[l.x, l.y, l.z, 0.0, lc[0], lc[1], lc[2], 0.0, am[0], am[1], am[2], 0.0]);
        // time.z: light depth per metre, so the shader can bias shadows in metres.
        u.extend_from_slice(&[f.time_s, if f.atmosphere && f.sky.is_day { 1.0 } else { 0.0 }, 1.0 / (2.0 * light_range), if f.atmosphere { 0.0 } else { 1.0 }]);
        u.extend_from_slice(&[f.bend.tx, f.bend.ty, f.bend.radius, f.bend.t, f.bend.lat_y, 0.0, 0.0, 0.0]);
        self.queue.write_buffer(&self.uni, 0, f32_bytes(&u));
        let thresh = (f.view_height / h as f32) * 3.0 / (2.0 * f.range);
        // Orthographic depth is linear: 0.5 at the focus, 1/(2 range) per world unit away from the camera.
        let per_unit = 1.0 / (2.0 * f.range);
        let (near, far) = (0.5 - 0.3 * f.view_height * per_unit, 0.5 + 1.4 * f.view_height * per_unit);
        let fog = f.sky.fog;
        let b = [
            snap.ex, snap.ey, w as f32, h as f32, thresh, if f.outline { 1.0 } else { 0.0 }, 0.0, 0.0,
            fog[0], fog[1], fog[2], if f.atmosphere { 1.0 } else { 0.0 },
            near, far, 0.55, 0.35,
        ];
        self.queue.write_buffer(&self.b_uni, 0, f32_bytes(&b));

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => {
                self.surface.configure(&self.device, &self.config);
                return Ok(()); // skip this frame
            }
        };
        let screen = frame.texture.create_view(&Default::default());
        let measure = self.poll_timing();
        let bg = if f.atmosphere {
            wgpu::Color { r: fog[0] as f64, g: fog[1] as f64, b: fog[2] as f64, a: 1.0 }
        } else {
            wgpu::Color { r: 0.02, g: 0.025, b: 0.04, a: 1.0 }
        };
        let t = self.targets.as_ref().expect("ensured");
        let tw = |i: u32| {
            self.timing.as_ref().filter(|_| measure).map(|tm| wgpu::RenderPassTimestampWrites {
                query_set: &tm.set,
                beginning_of_pass_write_index: Some(2 * i),
                end_of_pass_write_index: Some(2 * i + 1),
            })
        };
        let mut enc = self.device.create_command_encoder(&Default::default());

        let draw_list = |pass: &mut wgpu::RenderPass<'_>, list: &[(String, String)]| {
            for (mk, ik) in list {
                if let (Some(mb), Some(ib)) = (self.buffers.get(mk), self.buffers.get(ik)) {
                    if mb.count > 0 && ib.count > 0 {
                        pass.set_vertex_buffer(0, mb.buf.slice(..));
                        pass.set_vertex_buffer(1, ib.buf.slice(..));
                        pass.draw(0..mb.count, 0..ib.count);
                    }
                }
            }
        };
        let draw_all = |pass: &mut wgpu::RenderPass<'_>| {
            pass.set_vertex_buffer(1, self.one_instance.slice(..));
            for k in f.meshes {
                if let Some(b) = self.buffers.get(k).filter(|b| b.count > 0) {
                    pass.set_vertex_buffer(0, b.buf.slice(..));
                    pass.draw(0..b.count, 0..1);
                }
            }
            for (mk, ik) in f.instanced {
                if let (Some(mb), Some(ib)) = (self.buffers.get(mk), self.buffers.get(ik)) {
                    if mb.count > 0 && ib.count > 0 {
                        pass.set_vertex_buffer(0, mb.buf.slice(..));
                        pass.set_vertex_buffer(1, ib.buf.slice(..));
                        pass.draw(0..mb.count, 0..ib.count);
                    }
                }
            }
        };
        {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                timestamp_writes: tw(0),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            p.set_pipeline(&self.shadow_pipe);
            p.set_bind_group(0, &self.shadow_bind, &[]);
            draw_all(&mut p);
        }
        {
            let clear = |c: wgpu::Color, view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(c), store: wgpu::StoreOp::Store },
                })
            };
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                timestamp_writes: tw(1),
                color_attachments: &[
                    clear(bg, &t.color),
                    clear(wgpu::Color { r: 0.5, g: 0.5, b: 1.0, a: 1.0 }, &t.normal),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            p.set_pipeline(&self.scene_pipe);
            p.set_bind_group(0, &self.scene_bind, &[]);
            draw_all(&mut p);
            draw_list(&mut p, f.decor);
        }
        {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("upscale"),
                timestamp_writes: tw(2),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            p.set_pipeline(&self.blit_pipe);
            p.set_bind_group(0, &t.blit_bind, &[]);
            p.draw(0..3, 0..1);
        }
        if let Some(tm) = self.timing.as_ref().filter(|_| measure) {
            enc.resolve_query_set(&tm.set, 0..6, &tm.resolve, 0);
            enc.copy_buffer_to_buffer(&tm.resolve, 0, &tm.readback, 0, 48);
        }
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        if let Some(tm) = self.timing.as_ref().filter(|_| measure) {
            *tm.state.lock().expect("timing lock") = 1;
            let state = tm.state.clone();
            tm.readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                *state.lock().expect("timing lock") = if r.is_ok() { 2 } else { 0 };
            });
        }
        Ok(())
    }
}
