//! Flat-shaded triangle lists: 3 floats of position and 1 colour code per vertex.
//! The colour code is `palette + 32 * part` (32 palette colours); a negative code means "body, in the instance colour".
//! Parts are the limbs the vertex shader swings for the walk cycle.

/// Parts of a mesh. `Body` is never posed (vehicles, terrain); the others belong to figures and are moved by
/// `pose::pose_vertex` (and its WGSL port); `Sway` bends with the wind (grass).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Body = 0,
    LeftThigh = 1,
    RightThigh = 2,
    LeftArm = 3,
    RightArm = 4,
    Sway = 5,
    LeftShin = 6,
    RightShin = 7,
    /// Torso, pack, head and helmet: leans with the upper body.
    Torso = 8,
    /// Held in the right hand; follows the hand and keeps its own pitch.
    Rifle = 9,
    /// Neck, head and helmet: leans with the torso and nods around the neck.
    Head = 10,
    /// Forearms and hands: bend at the elbows.
    LeftForearm = 11,
    RightForearm = 12,
    /// Boots below the ankle: stay flat on the ground while the leg swings over them.
    LeftFoot = 13,
    RightFoot = 14,
}

/// Number of palette colours (the GPU palette's length).
pub const PALETTE_SIZE: u32 = 32;
/// Palette index that means "the instance's colour" on any part (team colours, selection).
pub const TEAM: u8 = 31;

/// The basic soldier's kit colours (palette 16-30).
pub mod kit {
    /// Tan uniform, and its darker shade (pockets, helmet rim).
    pub const UNIFORM: u8 = 16;
    pub const UNIFORM_DARK: u8 = 17;
    /// Plate carrier.
    pub const VEST: u8 = 18;
    /// Boot laces (light grey).
    pub const STRAP: u8 = 19;
    /// Pouches.
    pub const PACK: u8 = 20;
    pub const BOOT: u8 = 21;
    pub const GLOVE: u8 = 22;
    pub const BALACLAVA: u8 = 23;
    /// Goggle lens.
    pub const LENS: u8 = 24;
    /// Goggle frame and strap.
    pub const GOGGLE: u8 = 25;
    pub const HELMET: u8 = 26;
    pub const SKIN: u8 = 27;
    /// Boot cuffs and soles.
    pub const PAD: u8 = 28;
    pub const METAL: u8 = 29;
    /// Spare (the commander's rolled sleeve once used it).
    pub const COYOTE_DARK: u8 = 30;
}

/// The palette index of a colour code (negative codes use the instance colour and return 0 here).
pub fn palette_of(code: f32) -> u32 {
    if code < 0.0 { 0 } else { code as u32 % PALETTE_SIZE }
}

/// Colour code for a palette index on a part.
pub fn code(palette: u8, part: Part) -> f32 {
    (palette as u32 % PALETTE_SIZE + PALETTE_SIZE * part as u32) as f32
}

/// The part a colour code belongs to. Negative codes are the torso in the instance colour.
pub fn part_of(code: f32) -> Part {
    if code < 0.0 {
        return Part::Torso;
    }
    match (code / PALETTE_SIZE as f32).floor() as u32 {
        1 => Part::LeftThigh,
        2 => Part::RightThigh,
        3 => Part::LeftArm,
        4 => Part::RightArm,
        5 => Part::Sway,
        6 => Part::LeftShin,
        7 => Part::RightShin,
        8 => Part::Torso,
        9 => Part::Rifle,
        10 => Part::Head,
        11 => Part::LeftForearm,
        12 => Part::RightForearm,
        13 => Part::LeftFoot,
        14 => Part::RightFoot,
        _ => Part::Body,
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: f32,
}

#[derive(Default, Clone, Debug)]
pub struct MeshBuilder {
    pub vertices: Vec<Vertex>,
}

impl MeshBuilder {
    pub fn tri(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], color: f32) {
        for pos in [a, b, c] {
            self.vertices.push(Vertex { pos, color });
        }
    }

    pub fn quad(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], color: f32) {
        self.tri(a, b, c, color);
        self.tri(a, c, d, color);
    }

    pub fn cuboid(&mut self, lo: [f32; 3], hi: [f32; 3], color: f32) {
        let p = |x: f32, y: f32, z: f32| [x, y, z];
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        let (a, b, c, d) = (p(x0, y0, z0), p(x1, y0, z0), p(x1, y1, z0), p(x0, y1, z0));
        let (e, f, g, h) = (p(x0, y0, z1), p(x1, y0, z1), p(x1, y1, z1), p(x0, y1, z1));
        self.quad(a, d, c, b, color);
        self.quad(e, f, g, h, color);
        self.quad(a, b, f, e, color);
        self.quad(b, c, g, f, color);
        self.quad(c, d, h, g, color);
        self.quad(d, a, e, h, color);
    }

    /// Eight-triangle diamond.
    pub fn diamond(&mut self, c: [f32; 3], r: f32, color: f32) {
        let top = [c[0], c[1], c[2] + r];
        let bot = [c[0], c[1], c[2] - r];
        let e = [[c[0] + r, c[1], c[2]], [c[0], c[1] + r, c[2]], [c[0] - r, c[1], c[2]], [c[0], c[1] - r, c[2]]];
        for i in 0..4 {
            let (a, b) = (e[i], e[(i + 1) % 4]);
            self.tri(top, a, b, color);
            self.tri(bot, b, a, color);
        }
    }

    /// Flat ring in the z = 0 plane.
    pub fn ring(&mut self, c: [f32; 2], radius: f32, width: f32, segments: u32, color: f32) {
        let q = |a: f32, r: f32| [c[0] + a.cos() * r, c[1] + a.sin() * r, 0.0];
        for i in 0..segments {
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.quad(q(a0, radius - width), q(a1, radius - width), q(a1, radius + width), q(a0, radius + width), color);
        }
    }

    /// A tapered prism with elliptical ends: `sides` facets from the ring at `c0` (radii `r0`) to the ring at
    /// `c1` (radii `r1`), both rings horizontal, with flat caps. Limbs, torsos and heads are built from these.
    pub fn prism(&mut self, c0: [f32; 3], r0: [f32; 2], c1: [f32; 3], r1: [f32; 2], sides: u32, color: f32) {
        let ring = |c: [f32; 3], r: [f32; 2], k: u32| {
            let a = (k as f32 + 0.5) / sides as f32 * std::f32::consts::TAU;
            [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin(), c[2]]
        };
        for k in 0..sides {
            let (a0, a1) = (ring(c0, r0, k), ring(c0, r0, k + 1));
            let (b0, b1) = (ring(c1, r1, k), ring(c1, r1, k + 1));
            self.quad(a0, a1, b1, b0, color);
        }
        for k in 1..sides - 1 {
            self.tri(ring(c0, r0, 0), ring(c0, r0, k + 1), ring(c0, r0, k), color);
            self.tri(ring(c1, r1, 0), ring(c1, r1, k), ring(c1, r1, k + 1), color);
        }
    }

    /// A dome (half an ellipsoid) standing on the ellipse at `c` with radii `r`, `h` high.
    pub fn dome(&mut self, c: [f32; 3], r: [f32; 2], h: f32, sides: u32, rings: u32, color: f32) {
        let pt = |k: u32, j: u32| {
            let a = (k as f32 + 0.5) / sides as f32 * std::f32::consts::TAU;
            let e = j as f32 / rings as f32 * std::f32::consts::FRAC_PI_2;
            [c[0] + r[0] * e.cos() * a.cos(), c[1] + r[1] * e.cos() * a.sin(), c[2] + h * e.sin()]
        };
        for j in 0..rings {
            for k in 0..sides {
                if j + 1 == rings {
                    self.tri(pt(k, j), pt(k + 1, j), pt(0, rings), color);
                } else {
                    self.quad(pt(k, j), pt(k + 1, j), pt(k + 1, j + 1), pt(k, j + 1), color);
                }
            }
        }
    }

    pub fn bytes(&self) -> &[u8] {
        // Vertex is four f32 with no padding.
        unsafe { std::slice::from_raw_parts(self.vertices.as_ptr() as *const u8, self.vertices.len() * 16) }
    }
}

/// The basic soldier, converted from "low poly soldier" by Kolos Studios (CC-BY-4.0, see CREDITS.md) with
/// `scripts/glb_figure.py`: arms swung down onto our rig's pivots, each vertex given the part of its strongest bone,
/// materials mapped onto the kit palette, the M4 decimated into the right hand, a team armband on each upper arm.
pub fn figure() -> MeshBuilder {
    const FIG: &[u8] = include_bytes!("../assets/soldier.fig");
    let f = |i: usize| f32::from_le_bytes([FIG[i], FIG[i + 1], FIG[i + 2], FIG[i + 3]]);
    let vertices = (0..FIG.len() / 16).map(|k| Vertex { pos: [f(k * 16), f(k * 16 + 4), f(k * 16 + 8)], color: f(k * 16 + 12) }).collect();
    MeshBuilder { vertices }
}

/// The basic soldier, facing +y, about 1.78 m to the top of the helmet: chunky low-poly volumes after the art
/// reference. Tan uniform with thigh cargo pockets; plate carrier with front pouches and wings at the back; layered
/// shoulder pads; gloves with fingers and bare fingertips; boots with sole, heel, rounded toe cap, dark cuff and lace
/// stripes; dark balaclava with goggles; a tan helmet with a flared brim and neck guard; skin at the neck; a team
/// colour patch on each upper arm. Thighs hang from the hips (0.88 m), shins from the knees (0.46 m), upper arms
/// from the shoulders (1.40 m), forearms from the elbows (1.09 m), and the head nods around the neck (1.46 m); the
/// rifle is gripped in the right hand.
pub fn figure_kit() -> MeshBuilder {
    use kit::*;
    let mut f = MeshBuilder::default();
    let c = |p: u8, part: Part| code(p, part);
    for (s, thigh, shin, foot) in [(-1.0f32, Part::LeftThigh, Part::LeftShin, Part::LeftFoot), (1.0, Part::RightThigh, Part::RightShin, Part::RightFoot)] {
        let x = s * 0.105;
        // Thigh, full and tapering to the knee, with a cargo pocket on the outside.
        f.prism([x, 0.0, 0.48], [0.085, 0.092], [x, 0.0, 0.93], [0.118, 0.126], 8, c(UNIFORM, thigh));
        f.cuboid([x + s * 0.095 - 0.035, -0.06, 0.58], [x + s * 0.095 + 0.035, 0.07, 0.76], c(UNIFORM_DARK, thigh));
        f.cuboid([x + s * 0.095 - 0.037, -0.062, 0.74], [x + s * 0.095 + 0.037, 0.072, 0.775], c(UNIFORM, thigh));
        // Shin, then the boot: shaft, dark cuff, laces, foot with a raised heel and a rounded toe cap, sole.
        f.prism([x, 0.0, 0.22], [0.072, 0.078], [x, 0.0, 0.49], [0.086, 0.092], 8, c(UNIFORM, shin));
        f.prism([x, -0.005, 0.07], [0.078, 0.086], [x, -0.003, 0.15], [0.0806, 0.0877], 8, c(BOOT, foot));
        f.prism([x, -0.0035, 0.13], [0.0801, 0.0874], [x, 0.0, 0.24], [0.084, 0.09], 8, c(BOOT, shin));
        f.prism([x, 0.0, 0.215], [0.092, 0.1], [x, 0.0, 0.265], [0.096, 0.104], 8, c(PAD, shin));
        for k in 0..3 {
            let z = 0.11 + k as f32 * 0.035;
            f.cuboid([x - 0.03, 0.078, z], [x + 0.03, 0.094, z + 0.016], c(STRAP, shin));
        }
        f.cuboid([x - 0.074, -0.11, 0.025], [x + 0.074, 0.10, 0.11], c(BOOT, foot));
        f.prism([x, 0.115, 0.025], [0.074, 0.06], [x, 0.125, 0.095], [0.066, 0.05], 8, c(BOOT, foot));
        f.cuboid([x - 0.078, -0.115, 0.0], [x + 0.078, 0.19, 0.03], c(PAD, foot));
        f.cuboid([x - 0.07, -0.115, 0.03], [x + 0.07, -0.05, 0.06], c(PAD, foot));
    }
    // Hips and a broad torso in the uniform; skin at the neck.
    f.prism([0.0, 0.0, 0.82], [0.19, 0.125], [0.0, 0.0, 0.99], [0.185, 0.12], 10, c(UNIFORM, Part::Torso));
    f.prism([0.0, 0.0, 0.97], [0.18, 0.115], [0.0, 0.0, 1.42], [0.225, 0.14], 10, c(UNIFORM, Part::Torso));
    f.prism([0.0, 0.0, 1.38], [0.08, 0.075], [0.0, 0.0, 1.47], [0.07, 0.07], 8, c(SKIN, Part::Torso));
    // Plate carrier: front plate with three pouches, back plate with wings, cummerbund.
    f.cuboid([-0.19, 0.12, 1.0], [0.19, 0.175, 1.39], c(VEST, Part::Torso));
    f.cuboid([-0.19, -0.175, 1.0], [0.19, -0.12, 1.39], c(VEST, Part::Torso));
    f.prism([0.0, 0.0, 0.99], [0.235, 0.15], [0.0, 0.0, 1.2], [0.24, 0.152], 10, c(VEST, Part::Torso));
    for x in [-0.15f32, -0.045, 0.06] {
        f.cuboid([x, 0.175, 1.02], [x + 0.09, 0.22, 1.17], c(VEST, Part::Torso));
        f.cuboid([x - 0.003, 0.175, 1.15], [x + 0.093, 0.225, 1.18], c(METAL, Part::Torso));
    }
    for s in [-1.0f32, 1.0] {
        f.cuboid([s * 0.17 - 0.07, -0.205, 1.08], [s * 0.17 + 0.07, -0.15, 1.2], c(VEST, Part::Torso));
        f.cuboid([s * 0.12 - 0.035, -0.17, 1.38], [s * 0.12 + 0.035, 0.17, 1.42], c(VEST, Part::Torso));
    }
    // Head: balaclava, goggles, a tan helmet with a flared brim and neck guard.
    f.prism([0.0, 0.0, 1.44], [0.055, 0.055], [0.0, 0.0, 1.52], [0.06, 0.06], 8, c(BALACLAVA, Part::Head));
    f.prism([0.0, 0.01, 1.50], [0.092, 0.104], [0.0, 0.0, 1.67], [0.088, 0.098], 10, c(BALACLAVA, Part::Head));
    f.prism([0.0, 0.0, 1.595], [0.097, 0.11], [0.0, 0.0, 1.635], [0.097, 0.11], 10, c(GOGGLE, Part::Head));
    f.cuboid([-0.082, 0.094, 1.592], [0.082, 0.124, 1.64], c(LENS, Part::Head));
    f.prism([0.0, -0.015, 1.60], [0.14, 0.155], [0.0, -0.005, 1.66], [0.125, 0.14], 10, c(UNIFORM_DARK, Part::Head));
    f.dome([0.0, -0.005, 1.655], [0.125, 0.14], 0.125, 10, 3, c(HELMET, Part::Head));
    f.cuboid([-0.11, -0.165, 1.56], [0.11, -0.12, 1.64], c(HELMET, Part::Head));
    // Arms: layered shoulder pads with a team patch, a thick sleeve, gloves with fingers and bare fingertips.
    for (s, arm, fore) in [(-1.0f32, Part::LeftArm, Part::LeftForearm), (1.0, Part::RightArm, Part::RightForearm)] {
        let x = s * 0.238;
        f.prism([x, 0.0, 1.09], [0.064, 0.068], [x, 0.0, 1.40], [0.078, 0.082], 8, c(UNIFORM, arm));
        f.dome([x - s * 0.005, 0.0, 1.37], [0.09, 0.094], 0.075, 8, 2, c(UNIFORM, arm));
        f.prism([x, 0.0, 1.27], [0.086, 0.09], [x, 0.0, 1.33], [0.09, 0.094], 8, c(UNIFORM_DARK, arm));
        f.prism([x, 0.0, 1.21], [0.082, 0.086], [x, 0.0, 1.27], [0.086, 0.09], 8, c(UNIFORM, arm));
        f.cuboid([x + s * 0.088 - 0.01, -0.035, 1.22], [x + s * 0.088 + 0.01, 0.035, 1.3], c(TEAM, arm));
        f.prism([x, 0.0, 0.88], [0.056, 0.06], [x, 0.0, 1.12], [0.064, 0.068], 8, c(UNIFORM, fore));
        f.cuboid([x - 0.045, -0.045, 0.80], [x + 0.045, 0.055, 0.885], c(GLOVE, fore));
        for k in 0..4 {
            let fy = -0.03 + k as f32 * 0.025;
            f.cuboid([x - 0.044, fy, 0.75], [x + 0.044, fy + 0.02, 0.80], c(GLOVE, fore));
            f.cuboid([x - 0.04, fy + 0.002, 0.73], [x + 0.04, fy + 0.018, 0.75], c(SKIN, fore));
        }
    }
    // Rifle: stock, receiver, grip, magazine, handguard, barrel, optic.
    let r = |p: u8| code(p, Part::Rifle);
    f.cuboid([0.218, -0.20, 0.780], [0.258, 0.0, 0.855], r(METAL));
    f.cuboid([0.215, 0.0, 0.790], [0.261, 0.36, 0.870], r(METAL));
    f.cuboid([0.225, 0.02, 0.730], [0.251, 0.07, 0.790], r(VEST));
    f.cuboid([0.225, 0.12, 0.700], [0.251, 0.19, 0.790], r(VEST));
    f.cuboid([0.217, 0.36, 0.800], [0.259, 0.58, 0.860], r(UNIFORM_DARK));
    f.cuboid([0.230, 0.58, 0.818], [0.246, 0.77, 0.836], r(METAL));
    f.cuboid([0.223, 0.08, 0.870], [0.253, 0.2, 0.915], r(VEST));
    f
}

/// The command tablet held in the tablet pose: "Military Rugged PDA / Tactical Tablet" by Snupp (CC-BY-4.0, see
/// CREDITS.md), converted with `scripts/glb_prop.py`. It sits between the hands at `pose::TABLET_AT`, its face tilted
/// back towards the holder by `pose::TABLET_TILT`. Drawn as an unposed prop (part 0) with the commander's instance.
pub fn pda_held() -> MeshBuilder {
    use crate::pose::{TABLET_AT, TABLET_TILT};
    const FIG: &[u8] = include_bytes!("../assets/pda.fig");
    let f = |i: usize| f32::from_le_bytes([FIG[i], FIG[i + 1], FIG[i + 2], FIG[i + 3]]);
    let (s, c) = TABLET_TILT.sin_cos();
    // Right, up the face, out of the face (towards the holder's eyes).
    let (r, u, n) = (glam::Vec3::X, glam::Vec3::new(0.0, s, c), glam::Vec3::new(0.0, -c, s));
    let vertices = (0..FIG.len() / 16)
        .map(|k| {
            let (x, y, z) = (f(k * 16), f(k * 16 + 4), f(k * 16 + 8));
            Vertex { pos: (TABLET_AT + r * x + u * y + n * z).to_array(), color: f(k * 16 + 12) }
        })
        .collect();
    MeshBuilder { vertices }
}

/// The lighter figure for 70-250 m: same parts and proportions, a few boxes, the same colours.
pub fn figure_lo() -> MeshBuilder {
    use kit::*;
    let mut f = MeshBuilder::default();
    f.cuboid([-0.17, -0.09, 0.0], [-0.03, 0.09, 0.88], code(UNIFORM, Part::LeftThigh));
    f.cuboid([0.03, -0.09, 0.0], [0.17, 0.09, 0.88], code(UNIFORM, Part::RightThigh));
    f.cuboid([-0.19, -0.12, 0.85], [0.19, 0.12, 1.42], code(UNIFORM, Part::Torso));
    f.cuboid([-0.19, -0.13, 0.88], [0.19, 0.13, 0.98], code(STRAP, Part::Torso));
    f.cuboid([-0.12, -0.19, 0.86], [0.12, -0.115, 1.02], code(PACK, Part::Torso));
    f.cuboid([-0.08, -0.09, 1.42], [0.08, 0.1, 1.62], code(BALACLAVA, Part::Head));
    f.cuboid([-0.1, -0.11, 1.6], [0.1, 0.12, 1.74], code(HELMET, Part::Head));
    f.cuboid([-0.29, -0.06, 1.09], [-0.19, 0.06, 1.42], code(UNIFORM, Part::LeftArm));
    f.cuboid([0.19, -0.06, 1.09], [0.29, 0.06, 1.42], code(UNIFORM, Part::RightArm));
    f.cuboid([-0.29, -0.06, 0.77], [-0.19, 0.06, 1.12], code(UNIFORM, Part::LeftForearm));
    f.cuboid([0.19, -0.06, 0.77], [0.29, 0.06, 1.12], code(UNIFORM, Part::RightForearm));
    f.cuboid([0.21, -0.1, 0.78], [0.26, 0.72, 0.86], code(METAL, Part::Rifle));
    f
}

/// The far figure, for views where a soldier is a few pixels tall: uniform and pack, and a helmet in the team
/// colour, 24 triangles. It still leans and lies down with the poses.
pub fn figure_far() -> MeshBuilder {
    let mut f = MeshBuilder::default();
    f.cuboid([-0.2, -0.13, 0.0], [0.2, 0.13, 1.5], code(kit::UNIFORM, Part::Torso));
    f.cuboid([-0.12, -0.12, 1.5], [0.12, 0.14, 1.74], code(TEAM, Part::Head));
    f
}

/// A grass tuft: five blades fanning out, 0.35-0.55 m tall and wide enough to cover a pixel at
/// battlefield zoom, all swaying. The instance picks the colour.
pub fn grass_tuft() -> MeshBuilder {
    let mut g = MeshBuilder::default();
    let blades = [(0.0f32, 0.55f32), (1.3, 0.42), (2.5, 0.48), (3.8, 0.36), (5.0, 0.45)];
    for (a, h) in blades {
        let (s, c) = a.sin_cos();
        let (dx, dy) = (c * 0.1, s * 0.1);
        let tip = [s * 0.15, -c * 0.15, h];
        g.tri([-dx, -dy, 0.0], [dx, dy, 0.0], tip, code(0, Part::Sway));
    }
    g
}
