//! GPU side of the skeletal animation (AN-03): the compute pass that turns each soldier's instance record into
//! joint transforms (skin_cs.wgsl), and the skinned draw pipelines (`vs_skin` in scene.wgsl and shadow.wgsl).
//! The animation tables are uploaded once; each instance buffer drawn with a skinned mesh gets its own joint
//! transform buffer, filled every frame before the shadow pass.
use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::shaders::SKIN_CS_WGSL;
use crate::skin::{Anims, ANIM_BYTES};

/// Bytes per skinned vertex: position (3 f32), colour code (f32), 4 joints (u8), 4 weights (u8).
pub const SKIN_VERTEX_BYTES: u64 = 24;

fn bytes<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn entry(binding: u32, buf: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buf.as_entire_binding(),
    }
}

/// Joint transforms of one instance buffer.
struct Set {
    palette: wgpu::Buffer,
    params: wgpu::Buffer,
    compute: wgpu::BindGroup,
    scene: wgpu::BindGroup,
    shadow: wgpu::BindGroup,
    /// Instance buffer generation and capacity (in instances) these were made for.
    made_for: (u64, u32),
}

pub struct SkinGpu {
    pub scene_pipe: wgpu::RenderPipeline,
    pub shadow_pipe: wgpu::RenderPipeline,
    pub scene_bind: wgpu::BindGroup,
    pub shadow_bind: wgpu::BindGroup,
    compute_pipe: wgpu::ComputePipeline,
    skel: wgpu::Buffer,
    info: wgpu::Buffer,
    frames: wgpu::Buffer,
    map: wgpu::Buffer,
    joints_uni: wgpu::Buffer,
    joints: u32,
    sets: HashMap<String, Set>,
}

pub struct Shared<'a> {
    pub scene_mod: &'a wgpu::ShaderModule,
    pub shadow_mod: &'a wgpu::ShaderModule,
    pub uni: &'a wgpu::Buffer,
    pub shadow_view: &'a wgpu::TextureView,
    pub cmp: &'a wgpu::Sampler,
    pub instance_layout: wgpu::VertexBufferLayout<'static>,
}

impl SkinGpu {
    pub fn new(device: &wgpu::Device, s: Shared) -> SkinGpu {
        let anims = Anims::load(ANIM_BYTES);
        let (skel, info, map) = anims.gpu_tables();
        let init = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let storage = wgpu::BufferUsages::STORAGE;
        let joints = anims.joints.len() as u32;
        let skel = init("skin skeleton", bytes(&skel), storage);
        let info = init("skin clips", bytes(&info), storage);
        let frames = init("skin frames", bytes(&anims.frames_u32), storage);
        let map = init("skin pose map", bytes(&map), storage);
        let joints_uni = init(
            "skin joints",
            bytes(&[joints, 0, 0, 0]),
            wgpu::BufferUsages::UNIFORM,
        );

        const MESH: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 5 => Uint8x4, 6 => Unorm8x4];
        let layouts = [
            Some(wgpu::VertexBufferLayout {
                array_stride: SKIN_VERTEX_BYTES,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH,
            }),
            Some(s.instance_layout.clone()),
        ];
        let tri = wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        };
        let depth = |format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let rgba = Some(wgpu::ColorTargetState::from(
            wgpu::TextureFormat::Rgba8Unorm,
        ));
        let scene_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene skinned"),
            layout: None,
            vertex: wgpu::VertexState {
                module: s.scene_mod,
                entry_point: Some("vs_skin"),
                compilation_options: Default::default(),
                buffers: &layouts,
            },
            primitive: tri,
            depth_stencil: Some(depth(wgpu::TextureFormat::Depth24Plus)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: s.scene_mod,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[rgba.clone(), rgba],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow skinned"),
            layout: None,
            vertex: wgpu::VertexState {
                module: s.shadow_mod,
                entry_point: Some("vs_skin"),
                compilation_options: Default::default(),
                buffers: &layouts,
            },
            primitive: tri,
            depth_stencil: Some(depth(wgpu::TextureFormat::Depth32Float)),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene skinned"),
            layout: &scene_pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: s.uni.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(s.shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(s.cmp),
                },
            ],
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow skinned"),
            layout: &shadow_pipe.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: s.uni.as_entire_binding(),
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skin cs"),
            source: wgpu::ShaderSource::Wgsl(SKIN_CS_WGSL.into()),
        });
        let compute_pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("skin cs"),
            layout: None,
            module: &module,
            entry_point: Some("cs"),
            compilation_options: Default::default(),
            cache: None,
        });
        SkinGpu {
            scene_pipe,
            shadow_pipe,
            scene_bind,
            shadow_bind,
            compute_pipe,
            skel,
            info,
            frames,
            map,
            joints_uni,
            joints,
            sets: HashMap::new(),
        }
    }

    /// Fills the joint transforms of the `count` soldiers in instance buffer `key` for this frame (call before the
    /// passes that draw them). `generation` changes whenever the instance buffer is reallocated.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        key: &str,
        inst: &wgpu::Buffer,
        generation: u64,
        count: u32,
        time: f32,
    ) {
        if count == 0 {
            return;
        }
        let capacity = (inst.size() / crate::gpu::INSTANCE_BYTES as u64) as u32;
        let fresh = self
            .sets
            .get(key)
            .is_none_or(|s| s.made_for != (generation, capacity));
        if fresh {
            let palette = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("skin palette"),
                size: (capacity.max(1) * self.joints * 32) as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let params = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("skin params"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("skin cs"),
                layout: &self.compute_pipe.get_bind_group_layout(0),
                entries: &[
                    entry(0, &params),
                    entry(1, inst),
                    entry(2, &self.skel),
                    entry(3, &self.info),
                    entry(4, &self.frames),
                    entry(5, &self.map),
                    entry(6, &palette),
                ],
            });
            let draw = |pipe: &wgpu::RenderPipeline| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("skin palette"),
                    layout: &pipe.get_bind_group_layout(1),
                    entries: &[entry(0, &palette), entry(1, &self.joints_uni)],
                })
            };
            let (scene, shadow) = (draw(&self.scene_pipe), draw(&self.shadow_pipe));
            self.sets.insert(
                key.to_string(),
                Set {
                    palette,
                    params,
                    compute,
                    scene,
                    shadow,
                    made_for: (generation, capacity),
                },
            );
        }
        let set = &self.sets[key];
        let p: [u32; 4] = [time.to_bits(), count, self.joints, 0];
        queue.write_buffer(&set.params, 0, bytes(&p));
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("skin"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute_pipe);
        pass.set_bind_group(0, &set.compute, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }

    /// The palette bind group (group 1) for instance buffer `key`, if `update` has made one.
    pub fn bind(&self, key: &str, shadow: bool) -> Option<&wgpu::BindGroup> {
        self.sets
            .get(key)
            .map(|s| if shadow { &s.shadow } else { &s.scene })
    }
    /// For tests and the HUD: bytes of joint transforms held.
    pub fn palette_bytes(&self) -> u64 {
        self.sets.values().map(|s| s.palette.size()).sum()
    }
}
