//! AN-03: the skinning compute pass (skin_cs.wgsl) gives the transforms its CPU mirror (`skin::pose_instance`) does.
//! Runs on any native adapter wgpu finds (a software Vulkan one, lavapipe, is enough); skipped when there is none.
use scalia_render::gpu::INSTANCE_BYTES;
use scalia_render::shaders::SKIN_CS_WGSL;
use scalia_render::skin::{pose_instance, Anims, ANIM_BYTES};
use wgpu::util::DeviceExt;

fn bytes<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// Instance records exercising every path: gaits at angles, a crawl back, idles on the clock, a blend from the
/// previous pose, reload progress, an upper-body layer and one blending out, the drop keys, the dead.
fn records(time: f32) -> Vec<[f32; 12]> {
    let pack = |upper: f32, dir: f32| {
        upper + (dir.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU).min(0.9999)
    };
    vec![
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.3,
            0.2,
            0.9,
            1.0,
            1.0,
            -1000.0,
            pack(0.0, 0.4),
            0.0,
        ],
        [
            1.0,
            2.0,
            0.0,
            1.0,
            0.0,
            0.7,
            1.4,
            2.0,
            0.0,
            -1000.0,
            pack(0.0, -2.0),
            0.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.1,
            1.9,
            7.0,
            2.0,
            time - 0.1,
            pack(0.0, 1.0),
            0.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.45,
            0.4,
            32.0,
            5.0,
            -1000.0,
            pack(0.0, 3.0),
            0.0,
        ],
        [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.33, 0.0, 0.0, 0.0, -1000.0, 0.0, 0.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            -1.6,
            18.0,
            0.0,
            time - 0.2,
            0.0,
            0.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.25,
            1.2,
            1.0,
            1.0,
            -1000.0,
            pack(7.0, 0.0),
            0.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.25,
            1.2,
            1.0,
            1.0,
            -1000.0,
            pack(0.0, 0.0),
            7.0 * 64.0 + 40.0,
        ],
        [
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.5,
            -1.0,
            35.0,
            34.0,
            time - 0.15,
            0.0,
            0.0,
        ],
        [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 24.0, 24.0, -1000.0, 0.0, 0.0,
        ],
    ]
}

fn e(binding: u32, b: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: b.as_entire_binding(),
    }
}

#[test]
fn gpu_pose_matches_the_cpu_mirror() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: false,
        ..Default::default()
    })) else {
        eprintln!("no native GPU adapter: skipped");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("device");
    let an = Anims::load(ANIM_BYTES);
    let (skel, info, map) = an.gpu_tables();
    let time = 12.34f32;
    let recs = records(time);
    let n = recs.len() as u32;
    let flat: Vec<f32> = recs.iter().flatten().copied().collect();
    assert_eq!(flat.len() * 4, recs.len() * INSTANCE_BYTES);
    let init = |c: &[u8], u| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: c,
            usage: u,
        })
    };
    let st = wgpu::BufferUsages::STORAGE;
    let joints = an.joints.len() as u32;
    let params = init(
        bytes(&[time.to_bits(), n, joints, 0]),
        wgpu::BufferUsages::UNIFORM,
    );
    let inst = init(bytes(&flat), st);
    let (skel, info, frames, map) = (
        init(bytes(&skel), st),
        init(bytes(&info), st),
        init(bytes(&an.frames_u32), st),
        init(bytes(&map), st),
    );
    let size = (n * joints * 32) as u64;
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: st | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SKIN_CS_WGSL.into()),
    });
    let pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("cs"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[
            e(0, &params),
            e(1, &inst),
            e(2, &skel),
            e(3, &info),
            e(4, &frames),
            e(5, &map),
            e(6, &out),
        ],
    });
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let mut p = enc.begin_compute_pass(&Default::default());
        p.set_pipeline(&pipe);
        p.set_bind_group(0, &bind, &[]);
        p.dispatch_workgroups(1, 1, 1);
    }
    enc.copy_buffer_to_buffer(&out, 0, &read, 0, size);
    queue.submit([enc.finish()]);
    read.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let got: Vec<f32> = read
        .slice(..)
        .get_mapped_range()
        .expect("mapped")
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let mut worst = (0.0f32, 0usize, 0usize);
    for (i, rec) in recs.iter().enumerate() {
        let want = pose_instance(&an, rec, time);
        for (j, x) in want.iter().enumerate() {
            let at = (i * joints as usize + j) * 8;
            let q = glam::Quat::from_xyzw(got[at], got[at + 1], got[at + 2], got[at + 3]);
            let p = glam::Vec3::new(got[at + 4], got[at + 5], got[at + 6]);
            // Compare where the joint takes a point 20 cm out (rotation and position together).
            let probe = glam::Vec3::new(0.1, 0.1, 0.1);
            let err = (q * probe + p).distance(x.rot * probe + x.pos);
            if err > worst.0 {
                worst = (err, i, j);
            }
        }
    }
    eprintln!(
        "GPU vs CPU: worst {:.5} m (record {}, joint {})",
        worst.0, worst.1, worst.2
    );
    assert!(
        worst.0 < 0.002,
        "GPU pose differs from the CPU mirror by {:.4} m (record {}, joint {})",
        worst.0,
        worst.1,
        worst.2
    );
}

#[test]
fn skinned_pipelines_build_dispatch_and_draw() {
    // The game's own scene and shadow shaders, the skinned pipelines and their bind groups, a compute dispatch and
    // a draw into offscreen targets: any binding or layout mistake fails validation here.
    use scalia_render::shaders::{SCENE_WGSL, SHADOW_WGSL};
    use scalia_render::skin_gpu::{Shared, SkinGpu};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no native GPU adapter: skipped");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("device");
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu: {e}")));
    let module = |src: &str| {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(src.into()),
        })
    };
    let (scene_mod, shadow_mod) = (module(SCENE_WGSL), module(SHADOW_WGSL));
    let uni = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: scalia_render::gpu::UNIFORM_BYTES,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let tex = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let rt = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let shadow = tex(
        wgpu::TextureFormat::Depth32Float,
        rt | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let shadow_view = shadow.create_view(&Default::default());
    let cmp = device.create_sampler(&wgpu::SamplerDescriptor {
        compare: Some(wgpu::CompareFunction::Less),
        ..Default::default()
    });
    const INST: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
    let instance_layout = wgpu::VertexBufferLayout {
        array_stride: INSTANCE_BYTES as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &INST,
    };
    let mut skin = SkinGpu::new(
        &device,
        Shared {
            scene_mod: &scene_mod,
            shadow_mod: &shadow_mod,
            uni: &uni,
            shadow_view: &shadow_view,
            cmp: &cmp,
            instance_layout,
        },
    );
    let recs: Vec<f32> = records(3.0).iter().flatten().copied().collect();
    let st = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE;
    let inst = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytes(&recs),
        usage: st,
    });
    let mesh = scalia_render::skin::mesh_bytes(&|p| p);
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &mesh,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let count = (recs.len() / 12) as u32;
    let mut enc = device.create_command_encoder(&Default::default());
    skin.update(&device, &queue, &mut enc, "soldiers", &inst, 1, count, 3.0);
    let (color, normal, depth) = (
        tex(wgpu::TextureFormat::Rgba8Unorm, rt),
        tex(wgpu::TextureFormat::Rgba8Unorm, rt),
        tex(wgpu::TextureFormat::Depth24Plus, rt),
    );
    let (cv, nv, dv) = (
        color.create_view(&Default::default()),
        normal.create_view(&Default::default()),
        depth.create_view(&Default::default()),
    );
    let verts = (mesh.len() / 24) as u32;
    {
        let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        p.set_pipeline(&skin.shadow_pipe);
        p.set_bind_group(0, &skin.shadow_bind, &[]);
        p.set_bind_group(1, skin.bind("soldiers", true).unwrap(), &[]);
        p.set_vertex_buffer(0, vb.slice(..));
        p.set_vertex_buffer(1, inst.slice(..));
        p.draw(0..verts, 0..count);
    }
    {
        let att = |v| {
            Some(wgpu::RenderPassColorAttachment {
                view: v,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[att(&cv), att(&nv)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &dv,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        p.set_pipeline(&skin.scene_pipe);
        p.set_bind_group(0, &skin.scene_bind, &[]);
        p.set_bind_group(1, skin.bind("soldiers", false).unwrap(), &[]);
        p.set_vertex_buffer(0, vb.slice(..));
        p.set_vertex_buffer(1, inst.slice(..));
        p.draw(0..verts, 0..count);
    }
    queue.submit([enc.finish()]);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}
