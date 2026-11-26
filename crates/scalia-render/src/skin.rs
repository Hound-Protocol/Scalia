//! Skeletal animation (AN-03): the Mixamo-rigged soldier and the clips baked for it by `scripts/skin_bake.py`.
//!
//! `rigged.skin`: "SKN1", vertex count, joint count (u32), then a triangle list of vertices: position (3 f32,
//! figure frame: x right, y forward, z up, metres), colour code (f32, as `mesh::code`), 4 joint indices (u8),
//! 4 weights (u8, summing to 255).
//!
//! `anims.bin`: "ANM1", joint count, clip count (u32); per joint: name (32 bytes), parent (i32, -1 for the root,
//! parents come first), bind rotation (quaternion x y z w) and position in the figure frame, bone offset from
//! the parent (3 f32); per clip: name (32 bytes), frames (u32), loops (u32), frames per second, seconds,
//! horizontal root travel per playthrough (2 f32, taken out of the frames), byte offset into the frame data
//! (u32); then the frame data: per frame the root position (3 f32) and every joint's rotation local to its parent
//! (4 i16 each, snorm). The GPU reads the frame data as it is (`frames_u32`).
//!
//! This module is the CPU reference for the GPU path (`wgsl/skin.wgsl`): sampling, forward kinematics and skinning.
use glam::{Quat, Vec2, Vec3};

pub const SKIN_BYTES: &[u8] = include_bytes!("../assets/rigged.skin");
pub const ANIM_BYTES: &[u8] = include_bytes!("../assets/anims.bin");

/// The baked clips, loaded once.
pub fn anims() -> &'static Anims {
    static A: std::sync::OnceLock<Anims> = std::sync::OnceLock::new();
    A.get_or_init(|| Anims::load(ANIM_BYTES))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkinVertex {
    pub pos: [f32; 3],
    pub color: f32,
    pub joints: [u8; 4],
    pub weights: [u8; 4],
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub name: String,
    pub parent: i32,
    pub bind_rot: Quat,
    pub bind_pos: Vec3,
    /// Offset from the parent joint, in the parent's frame (the root's comes from the clip).
    pub bone: Vec3,
}

#[derive(Clone, Debug)]
pub struct ClipInfo {
    pub name: String,
    pub frames: u32,
    pub looped: bool,
    pub fps: f32,
    pub seconds: f32,
    /// Ground the clip covers per playthrough (m, x right, y forward); the game moves the soldier instead.
    pub travel: Vec2,
    /// Offset of the first frame in the frame data, in u32 words.
    pub word: u32,
}

/// A joint transform: rotation and position.
#[derive(Clone, Copy, Debug)]
pub struct Xf {
    pub rot: Quat,
    pub pos: Vec3,
}

impl Xf {
    pub const ID: Xf = Xf {
        rot: Quat::IDENTITY,
        pos: Vec3::ZERO,
    };
    pub fn mul(self, o: Xf) -> Xf {
        Xf {
            rot: self.rot * o.rot,
            pos: self.pos + self.rot * o.pos,
        }
    }
    pub fn inverse(self) -> Xf {
        let r = self.rot.inverse();
        Xf {
            rot: r,
            pos: -(r * self.pos),
        }
    }
    pub fn apply(self, v: Vec3) -> Vec3 {
        self.rot * v + self.pos
    }
}

pub struct Anims {
    pub joints: Vec<Joint>,
    pub clips: Vec<ClipInfo>,
    /// Per pose id (`pose_clips`, resolved once): how it plays and its clip indices.
    pub poses: Vec<(Play, Vec<usize>)>,
    /// Joints from the spine up (`upper_body`).
    pub upper: Vec<bool>,
    /// Frame data as u32 words (root position as f32 bits, then i16 pairs).
    pub frames_u32: Vec<u32>,
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}
fn name_at(b: &[u8], o: usize) -> String {
    let raw = &b[o..o + 32];
    String::from_utf8_lossy(&raw[..raw.iter().position(|&c| c == 0).unwrap_or(32)]).into_owned()
}

/// Words per frame of a clip: root position (3) and two per joint rotation.
pub fn frame_words(joints: usize) -> usize {
    3 + 2 * joints
}

impl Anims {
    pub fn load(b: &[u8]) -> Anims {
        assert_eq!(&b[0..4], b"ANM1", "not an anims.bin");
        let (nj, nc) = (u32_at(b, 4) as usize, u32_at(b, 8) as usize);
        let mut o = 12;
        let joints = (0..nj)
            .map(|_| {
                let j = Joint {
                    name: name_at(b, o),
                    parent: u32_at(b, o + 32) as i32,
                    bind_rot: Quat::from_xyzw(
                        f32_at(b, o + 36),
                        f32_at(b, o + 40),
                        f32_at(b, o + 44),
                        f32_at(b, o + 48),
                    ),
                    bind_pos: Vec3::new(f32_at(b, o + 52), f32_at(b, o + 56), f32_at(b, o + 60)),
                    bone: Vec3::new(f32_at(b, o + 64), f32_at(b, o + 68), f32_at(b, o + 72)),
                };
                o += 76;
                j
            })
            .collect();
        let clips: Vec<ClipInfo> = (0..nc)
            .map(|_| {
                let c = ClipInfo {
                    name: name_at(b, o),
                    frames: u32_at(b, o + 32),
                    looped: u32_at(b, o + 36) != 0,
                    fps: f32_at(b, o + 40),
                    seconds: f32_at(b, o + 44),
                    travel: Vec2::new(f32_at(b, o + 48), f32_at(b, o + 52)),
                    word: u32_at(b, o + 56) / 4,
                };
                o += 60;
                c
            })
            .collect();
        let frames_u32 = b[o..]
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        let mut a = Anims {
            joints,
            clips,
            poses: Vec::new(),
            upper: Vec::new(),
            frames_u32,
        };
        a.poses = (0..POSE_IDS)
            .map(|p| {
                let (play, names) = pose_clips(p);
                (
                    play,
                    names
                        .iter()
                        .map(|n| a.clip(n).unwrap_or_else(|| panic!("pose {p}: no clip {n}")))
                        .collect(),
                )
            })
            .collect();
        a.upper = a.upper_body();
        a
    }

    pub fn clip(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    fn frame(&self, clip: usize, f: usize) -> (Vec3, impl Fn(usize) -> Quat + '_) {
        let c = &self.clips[clip];
        let base = c.word as usize + f * frame_words(self.joints.len());
        let w = &self.frames_u32;
        let root = Vec3::new(
            f32::from_bits(w[base]),
            f32::from_bits(w[base + 1]),
            f32::from_bits(w[base + 2]),
        );
        let rot = move |j: usize| {
            let (a, b) = (w[base + 3 + 2 * j], w[base + 4 + 2 * j]);
            let s = |v: u32| ((v as u16) as i16 as f32 / 32767.0).max(-1.0);
            Quat::from_xyzw(s(a), s(a >> 16), s(b), s(b >> 16)).normalize()
        };
        (root, rot)
    }

    /// Local transform of joint `j` of `clip` at `t` seconds (a loop wraps; a one-off holds its last frame).
    pub fn sample_joint(&self, clip: usize, t: f32, j: usize) -> Xf {
        let c = &self.clips[clip];
        let n = c.frames as usize;
        let x = t * c.fps;
        let (i0, i1, w) = if c.looped {
            let x = x.rem_euclid(n as f32);
            let i0 = x.floor() as usize % n;
            (i0, (i0 + 1) % n, x - x.floor())
        } else {
            let x = x.clamp(0.0, (n - 1) as f32);
            let i0 = (x.floor() as usize).min(n - 1);
            (i0, (i0 + 1).min(n - 1), x - x.floor())
        };
        let (r0, q0) = self.frame(clip, i0);
        let (r1, q1) = self.frame(clip, i1);
        let (a, b) = (q0(j), q1(j));
        let rot = a.lerp(if a.dot(b) < 0.0 { -b } else { b }, w).normalize();
        let pos = if self.joints[j].parent < 0 {
            r0.lerp(r1, w)
        } else {
            self.joints[j].bone
        };
        Xf { rot, pos }
    }

    /// Local joint transforms of `clip` at `t` seconds.
    pub fn sample(&self, clip: usize, t: f32) -> Vec<Xf> {
        (0..self.joints.len())
            .map(|j| self.sample_joint(clip, t, j))
            .collect()
    }

    /// Joint transforms in the figure frame (forward kinematics; parents come first).
    pub fn world(&self, local: &[Xf]) -> Vec<Xf> {
        let mut w: Vec<Xf> = Vec::with_capacity(local.len());
        for (j, l) in local.iter().enumerate() {
            let p = self.joints[j].parent;
            w.push(if p < 0 { *l } else { w[p as usize].mul(*l) });
        }
        w
    }

    /// Skinning transforms: from the bind pose to `world`.
    pub fn skinning(&self, world: &[Xf]) -> Vec<Xf> {
        world
            .iter()
            .zip(&self.joints)
            .map(|(w, j)| {
                w.mul(
                    Xf {
                        rot: j.bind_rot,
                        pos: j.bind_pos,
                    }
                    .inverse(),
                )
            })
            .collect()
    }
}

/// Blends two sets of local transforms (`w` towards `b`).
pub fn blend(a: &[Xf], b: &[Xf], w: f32) -> Vec<Xf> {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let q = if x.rot.dot(y.rot) < 0.0 {
                -y.rot
            } else {
                y.rot
            };
            Xf {
                rot: x.rot.lerp(q, w).normalize(),
                pos: x.pos.lerp(y.pos, w),
            }
        })
        .collect()
}

pub fn load_mesh(b: &[u8]) -> Vec<SkinVertex> {
    assert_eq!(&b[0..4], b"SKN1", "not a rigged.skin");
    let n = u32_at(b, 4) as usize;
    (0..n)
        .map(|k| {
            let o = 12 + 24 * k;
            SkinVertex {
                pos: [f32_at(b, o), f32_at(b, o + 4), f32_at(b, o + 8)],
                color: f32_at(b, o + 12),
                joints: b[o + 16..o + 20].try_into().unwrap(),
                weights: b[o + 20..o + 24].try_into().unwrap(),
            }
        })
        .collect()
}

/// A vertex moved by the skinning transforms (linear blend skinning).
pub fn skin_vertex(v: &SkinVertex, skin: &[Xf]) -> Vec3 {
    let p = Vec3::from(v.pos);
    (0..4)
        .map(|i| skin[v.joints[i] as usize].apply(p) * (v.weights[i] as f32 / 255.0))
        .sum()
}

/// How a game pose plays its clips (skin_cs.wgsl).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Over the gait phase; standing still, on the clock.
    Loop = 0,
    /// Over the phase, once (reload, falls, the drop).
    Once = 1,
    /// The last frame (the dead).
    Hold = 2,
    /// Always on the clock (idles).
    Clock = 3,
}

/// The clips each game pose (pose.rs ids) plays: how, and one clip, two (ahead, back) or eight (each eighth of a
/// turn towards the left, from ahead). Poses with no clip of their own borrow the nearest.
pub fn pose_clips(pose: u32) -> (Play, Vec<&'static str>) {
    const WALK: [&str; 8] = [
        "walk_f", "walk_fl", "walk_l", "walk_bl", "walk_b", "walk_br", "walk_r", "walk_fr",
    ];
    const RUN: [&str; 8] = [
        "run_f", "run_fl", "run_l", "run_bl", "run_b", "run_br", "run_r", "run_fr",
    ];
    const SPRINT: [&str; 8] = [
        "sprint_f",
        "sprint_fl",
        "sprint_l",
        "sprint_bl",
        "sprint_b",
        "sprint_br",
        "sprint_r",
        "sprint_fr",
    ];
    const CWALK: [&str; 8] = [
        "cwalk_f", "cwalk_fl", "cwalk_l", "cwalk_bl", "cwalk_b", "cwalk_br", "cwalk_r", "cwalk_fr",
    ];
    use Play::*;
    match pose {
        1 => (Loop, WALK.to_vec()),
        2 => (Loop, RUN.to_vec()),
        7 => (Loop, SPRINT.to_vec()),
        11 => (Loop, CWALK.to_vec()),
        32 => (Loop, vec!["crawl_f", "crawl_b"]),
        3 | 20 | 22 | 23 => (Clock, vec!["crouch"]),
        14 => (Clock, vec!["crouch_aim"]),
        4 | 6 | 12 | 13 | 15 | 16 | 30 | 37 => (Clock, vec!["idle_aim"]),
        5 => (Clock, vec!["prone"]),
        36 => (Clock, vec!["prone_fire"]),
        8 => (Clock, vec!["cover"]),
        17 => (Once, vec!["throw"]),
        18 => (Once, vec!["reload"]),
        21 => (Clock, vec!["wounded"]),
        24 => (Hold, vec!["death_back"]),
        27 => (Hold, vec!["death_front"]),
        25 => (Once, vec!["death_front"]),
        26 => (Once, vec!["death_back"]),
        31 => (Once, vec!["brake"]),
        33 | 34 | 35 => (Once, vec!["crouch_to_prone"]),
        _ => (Clock, vec!["idle"]),
    }
}

/// Highest pose id (pose.rs).
pub const POSE_IDS: u32 = 38;

impl Anims {
    /// Joints from the spine up: the upper-body layer replaces these.
    pub fn upper_body(&self) -> Vec<bool> {
        let mut up = vec![false; self.joints.len()];
        for j in 0..self.joints.len() {
            let p = self.joints[j].parent;
            up[j] = self.joints[j].name == "Spine" || (p >= 0 && up[p as usize]);
        }
        up
    }

    /// GPU tables for skin_cs.wgsl: the skeleton (3 vec4 per joint), the clip index (2 vec4u per clip) and the
    /// pose map (3 vec4u per pose id).
    pub fn gpu_tables(&self) -> (Vec<f32>, Vec<u32>, Vec<u32>) {
        let up = &self.upper;
        let mut skel = Vec::new();
        for (j, joint) in self.joints.iter().enumerate() {
            let inv = Xf {
                rot: joint.bind_rot,
                pos: joint.bind_pos,
            }
            .inverse();
            skel.extend_from_slice(&[inv.rot.x, inv.rot.y, inv.rot.z, inv.rot.w]);
            skel.extend_from_slice(&[inv.pos.x, inv.pos.y, inv.pos.z, joint.parent as f32]);
            skel.extend_from_slice(&[
                joint.bone.x,
                joint.bone.y,
                joint.bone.z,
                if up[j] { 1.0 } else { 0.0 },
            ]);
        }
        let mut info = Vec::new();
        for c in &self.clips {
            info.extend_from_slice(&[
                c.word,
                c.frames,
                c.looped as u32,
                c.fps.to_bits(),
                c.seconds.to_bits(),
                0,
                0,
                0,
            ]);
        }
        let mut map = Vec::new();
        for pose in 0..POSE_IDS {
            let (play, clips) = pose_clips(pose);
            let ids: Vec<u32> = clips
                .iter()
                .map(|n| {
                    self.clip(n)
                        .unwrap_or_else(|| panic!("pose {pose}: no clip {n}"))
                        as u32
                })
                .collect();
            let mut row = [0u32; 12];
            row[0] = play as u32;
            row[1] = ids.len() as u32;
            for (k, id) in ids.iter().enumerate() {
                row[2 + k] = *id;
            }
            map.extend_from_slice(&row);
        }
        (skel, info, map)
    }
}

/// The game's rifle (the figure's rifle part) fixed to the right hand: placed so that, in the aiming clip, it runs
/// from the right palm towards the left palm (the support hand on the handguard), then carried by the hand bone
/// in every clip. Returned in the bind pose, weighted fully to the right hand.
pub fn rifle(anims: &Anims) -> Vec<SkinVertex> {
    use crate::mesh::{figure, part_of, Part};
    let rh = anims
        .joints
        .iter()
        .position(|x| x.name == "RightHand")
        .expect("RightHand");
    // Rifle to hand, then the hand where the bind pose has it.
    let place = Xf {
        rot: anims.joints[rh].bind_rot,
        pos: anims.joints[rh].bind_pos,
    }
    .mul(rifle_place(anims));
    figure()
        .vertices
        .iter()
        .filter(|v| v.color >= 0.0 && part_of(v.color) == Part::Rifle)
        .map(|v| SkinVertex {
            pos: place.apply(Vec3::from(v.pos)).into(),
            color: v.color,
            joints: [rh as u8, 0, 0, 0],
            weights: [255, 0, 0, 0],
        })
        .collect()
}

fn rifle_place_with(anims: &Anims, w: &[Xf], rot: glam::Mat3, grip: Vec3) -> Xf {
    let rh = anims
        .joints
        .iter()
        .position(|x| x.name == "RightHand")
        .expect("RightHand");
    let at = Xf {
        rot: Quat::from_mat3(&rot),
        pos: grip - rot * crate::pose::RIFLE_GRIP,
    };
    w[rh].inverse().mul(at)
}

/// The rifle's rest-pose points to the right hand's frame (see `rifle`).
pub fn rifle_place(anims: &Anims) -> Xf {
    use crate::pose::{RIFLE_GRIP, RIFLE_GUARD};
    let j = |n: &str| {
        anims
            .joints
            .iter()
            .position(|x| x.name == n)
            .unwrap_or_else(|| panic!("no joint {n}"))
    };
    let (rh, rm, lh, lm) = (
        j("RightHand"),
        j("RightHandMiddle1"),
        j("LeftHand"),
        j("LeftHandMiddle1"),
    );
    let w = anims.world(&anims.sample(anims.clip("idle_aim").expect("idle_aim"), 0.0));
    let grip = (w[rh].pos + w[rm].pos) * 0.5;
    let support = (w[lh].pos + w[lm].pos) * 0.5;
    let frame = |fwd: Vec3| {
        let up = (Vec3::Z - fwd * fwd.dot(Vec3::Z)).normalize();
        glam::Mat3::from_cols(fwd, up, fwd.cross(up))
    };
    let rot = frame((support - grip).normalize())
        * frame((RIFLE_GUARD - RIFLE_GRIP).normalize()).transpose();
    rifle_place_with(anims, &w, rot, grip)
}

/// The skinned soldier with the rifle in hand.
pub fn soldier(anims: &Anims) -> Vec<SkinVertex> {
    let mut m = load_mesh(SKIN_BYTES);
    m.extend(rifle(anims));
    m
}

/// The skinned soldier's vertex data for the GPU (`Gpu::upload_skinned`), rifle included, with palette colours
/// swapped by `recolour` (e.g. the enemy's red kit).
pub fn mesh_bytes(recolour: &dyn Fn(u32) -> u32) -> Vec<u8> {
    let mut b = Vec::new();
    for v in soldier(&Anims::load(ANIM_BYTES)) {
        let mut c = v.color;
        if c >= 0.0 {
            let (part, pal) = (
                c as u32 / crate::mesh::PALETTE_SIZE,
                c as u32 % crate::mesh::PALETTE_SIZE,
            );
            c = (part * crate::mesh::PALETTE_SIZE + recolour(pal)) as f32;
        }
        for f in [v.pos[0], v.pos[1], v.pos[2], c] {
            b.extend_from_slice(&f.to_le_bytes());
        }
        b.extend_from_slice(&v.joints);
        b.extend_from_slice(&v.weights);
    }
    b
}

/// One soldier's instance record (scene.wgsl `ia`, `ib`, `ic` as 12 floats) decoded as skin_cs.wgsl does.
struct Decoded {
    phase: f32,
    reload_phase: f32,
    w: f32,
    layer: f32,
    dir: f32,
    pose: u32,
    prev: u32,
    still: bool,
    clock: f32,
    drop_keys: bool,
    drop_t: f32,
    upper_id: u32,
    lb_code: f32,
    lb_w: f32,
}

fn decode(rec: &[f32; 12], time: f32) -> Decoded {
    let (b, d) = (&rec[4..8], &rec[8..12]);
    let fract = |x: f32| x - x.floor();
    let phase = if b[2] < 0.0 {
        fract(b[1])
    } else if b[2] > 0.0 {
        fract(time * b[2] + b[1])
    } else {
        0.0
    };
    let turn = fract(d[2]) * std::f32::consts::TAU;
    let (pose, prev) = (b[3] as u32, d[0] as u32);
    let w = ((time - d[1]) / 0.35).clamp(0.0, 1.0);
    let key = |p: u32, other: f32| match p {
        34 => 0.45,
        35 => 0.75,
        _ => other,
    };
    let (lb_code, lb_w) = if d[3] >= 1.0 {
        let whole = d[3].floor();
        let code = (whole / 64.0).floor();
        (code, (whole - code * 64.0) / 63.0)
    } else {
        (0.0, 0.0)
    };
    Decoded {
        phase,
        reload_phase: (-b[2] - 1.0).clamp(0.0, 1.0),
        w,
        layer: d[2].floor(),
        dir: if turn > std::f32::consts::PI {
            turn - std::f32::consts::TAU
        } else {
            turn
        },
        pose,
        prev,
        still: b[2] == 0.0,
        clock: time + b[1] * 97.0,
        drop_keys: matches!(pose, 34 | 35) || matches!(prev, 34 | 35),
        drop_t: key(prev, 0.0) + (key(pose, 1.0) - key(prev, 0.0)) * w,
        upper_id: (d[2].floor() - 1.0).max(0.0) as u32,
        lb_code,
        lb_w,
    }
}

fn mix(a: Xf, b: Xf, w: f32) -> Xf {
    let q = if a.rot.dot(b.rot) < 0.0 {
        -b.rot
    } else {
        b.rot
    };
    Xf {
        rot: a.rot.lerp(q, w).normalize(),
        pos: a.pos.lerp(b.pos, w),
    }
}

/// Joint `j` (local) of game pose `pose` at `phase` towards `dir`, as skin_cs.wgsl `pose_joint`.
fn pose_joint(an: &Anims, k: &Decoded, pose: u32, phase: f32, dir: f32, j: usize) -> Xf {
    let (play, ids) = &an.poses[(pose as usize).min(an.poses.len() - 1)];
    let play = *play;
    let (a, b, w) = if ids.len() == 8 {
        let x = dir.rem_euclid(std::f32::consts::TAU) / (std::f32::consts::TAU / 8.0);
        let i = (x.floor() as usize) % 8;
        (ids[i], ids[(i + 1) % 8], x - x.floor())
    } else if ids.len() == 2 && dir.cos() < 0.0 {
        (ids[1], ids[1], 0.0)
    } else {
        (ids[0], ids[0], 0.0)
    };
    let on_clock = play == Play::Clock || (play == Play::Loop && k.still);
    let at = |c: usize| {
        if play == Play::Hold {
            1.0e6
        } else if on_clock {
            k.clock
        } else {
            phase * an.clips[c].seconds
        }
    };
    let xa = an.sample_joint(a, at(a), j);
    if w > 0.0 {
        mix(xa, an.sample_joint(b, at(b), j), w)
    } else {
        xa
    }
}

/// Joint `j` (local) of the soldier, as one iteration of skin_cs.wgsl `cs`.
fn soldier_joint(an: &Anims, k: &Decoded, upper: bool, j: usize) -> Xf {
    let base_phase = if k.pose == 18 {
        k.reload_phase
    } else {
        k.phase
    };
    let prev_phase = if k.prev == 18 {
        k.reload_phase
    } else {
        k.phase
    };
    let upper_phase = if k.upper_id == 18 {
        k.reload_phase
    } else {
        k.phase
    };
    let mut x = if k.drop_keys {
        let c = an.poses[34].1[0];
        an.sample_joint(c, k.drop_t * an.clips[c].seconds, j)
    } else {
        let cur = pose_joint(an, k, k.pose, base_phase, k.dir, j);
        if k.w < 1.0 {
            mix(pose_joint(an, k, k.prev, prev_phase, k.dir, j), cur, k.w)
        } else {
            cur
        }
    };
    let had_was = k.lb_w > 0.0 && k.lb_code >= 1.0;
    if upper && k.layer >= 1.0 {
        let up = pose_joint(an, k, k.upper_id, upper_phase, 0.0, j);
        let was = if had_was {
            pose_joint(an, k, k.lb_code as u32 - 1, k.phase, 0.0, j)
        } else {
            x
        };
        x = if k.lb_w > 0.0 {
            mix(was, up, 1.0 - k.lb_w)
        } else {
            up
        };
    } else if upper && had_was {
        x = mix(
            x,
            pose_joint(an, k, k.lb_code as u32 - 1, k.phase, 0.0, j),
            k.lb_w,
        );
    }
    x
}

/// CPU mirror of skin_cs.wgsl: the skinning transforms of one soldier from its instance record (scene.wgsl `ia`,
/// `ib`, `ic` as 12 floats) at `time`. The GPU pass is tested against it (tests/skin_gpu.rs).
pub fn pose_instance(an: &Anims, rec: &[f32; 12], time: f32) -> Vec<Xf> {
    let k = decode(rec, time);
    let up = &an.upper;
    let local: Vec<Xf> = (0..an.joints.len())
        .map(|j| soldier_joint(an, &k, up[j], j))
        .collect();
    an.skinning(&an.world(&local))
}

/// Where one joint is (figure frame) for the soldier with instance record `rec` at `time`: only its chain from
/// the root is posed (cheap enough for every soldier every frame).
pub fn joint_at(an: &Anims, rec: &[f32; 12], time: f32, joint: usize) -> Xf {
    let k = decode(rec, time);
    let mut chain = [0usize; 16];
    let (mut n, mut j) = (0, joint as i32);
    while j >= 0 && n < chain.len() {
        chain[n] = j as usize;
        n += 1;
        j = an.joints[j as usize].parent;
    }
    chain[..n]
        .iter()
        .rev()
        .fold(Xf::ID, |w, &j| w.mul(soldier_joint(an, &k, an.upper[j], j)))
}

/// The rifle's muzzle and bore (figure frame) for the soldier with instance record `rec` at `time`, as drawn:
/// the rifle rides the right hand (`rifle`).
pub fn muzzle(an: &Anims, rec: &[f32; 12], time: f32) -> (Vec3, Vec3) {
    static PLACE: std::sync::OnceLock<(usize, Xf)> = std::sync::OnceLock::new();
    let (rh, place) = *PLACE.get_or_init(|| {
        (
            an.joints
                .iter()
                .position(|x| x.name == "RightHand")
                .expect("RightHand"),
            rifle_place(an),
        )
    });
    let hand = joint_at(an, rec, time, rh);
    let at = |v: Vec3| hand.apply(place.apply(v));
    let (tip, breech) = (at(crate::pose::RIFLE.muzzle), at(crate::pose::RIFLE.breech));
    (tip, (tip - breech).normalize_or_zero())
}

/// Ground one cycle of `pose` covers moving in `dir` (m): the clips' own travel, between the two nearest direction
/// clips (AN-03). The game sets cadence from it (speed / stride), so the planted feet keep pace with the ground.
/// Poses without moving clips fall back to the procedural stride.
pub fn stride_m_dir(pose: crate::pose::Pose, dir: f32) -> f32 {
    let an = anims();
    let (_, ids) = &an.poses[(pose as u32 as usize).min(an.poses.len() - 1)];
    let len = |i: usize| an.clips[ids[i]].travel.length();
    match ids.len() {
        8 => {
            let x = dir.rem_euclid(std::f32::consts::TAU) / (std::f32::consts::TAU / 8.0);
            let i = (x.floor() as usize) % 8;
            len(i) + (len((i + 1) % 8) - len(i)) * (x - x.floor())
        }
        2 => len(if dir.cos() < 0.0 { 1 } else { 0 }),
        _ if len(0) > 0.1 => len(0),
        _ => crate::pose::stride_m_dir(pose, dir),
    }
}

/// Metres one cycle of `pose`'s clip carries a soldier walking straight ahead (see `stride_m_dir`).
pub fn stride_m(pose: crate::pose::Pose) -> f32 {
    stride_m_dir(pose, 0.0)
}
