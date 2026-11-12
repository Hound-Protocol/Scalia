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
}

/// Number of palette colours (the GPU palette's length).
pub const PALETTE_SIZE: u32 = 32;
/// Palette index that means "the instance's colour" on any part (team colours, selection).
pub const TEAM: u8 = 31;

/// Soldier gear colours (palette 16-30).
pub mod kit {
    pub const UNIFORM: u8 = 16;
    pub const UNIFORM_DARK: u8 = 17;
    pub const VEST: u8 = 18;
    pub const STRAP: u8 = 19;
    pub const PACK: u8 = 20;
    pub const BOOT: u8 = 21;
    pub const GLOVE: u8 = 22;
    pub const BALACLAVA: u8 = 23;
    pub const LENS: u8 = 24;
    pub const GOGGLE: u8 = 25;
    pub const HELMET: u8 = 26;
    pub const SKIN: u8 = 27;
    pub const PAD: u8 = 28;
    pub const METAL: u8 = 29;
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

/// Low-poly soldier facing +y, 1.76 m to the top of the helmet, in the faceted style of the art reference:
/// tapered limbs and torso, plate carrier with magazine pouches, coyote pack, knee pads, gloves, balaclava,
/// goggles and a domed helmet with a band in the instance (team) colour. Thighs hang from the hips (0.88 m), shins
/// from the knees (0.46 m), upper arms from the shoulders (1.40 m), forearms from the elbows (1.14 m), and the head
/// nods around the neck (1.46 m); the rifle is gripped in the right hand.
pub fn figure() -> MeshBuilder {
    use kit::*;
    let mut f = MeshBuilder::default();
    let c = |p: u8, part: Part| code(p, part);
    for (s, thigh, shin) in [(-1.0f32, Part::LeftThigh, Part::LeftShin), (1.0, Part::RightThigh, Part::RightShin)] {
        let x = s * 0.10;
        f.prism([x, 0.0, 0.50], [0.066, 0.072], [x, 0.0, 0.92], [0.088, 0.094], 8, c(UNIFORM, thigh));
        f.cuboid([x + s * 0.06 - 0.02, -0.055, 0.60], [x + s * 0.06 + 0.075 - 0.02, 0.055, 0.75], c(UNIFORM_DARK, thigh));
        f.prism([x, 0.0, 0.13], [0.054, 0.06], [x, 0.0, 0.48], [0.066, 0.072], 8, c(UNIFORM, shin));
        f.cuboid([x - 0.068, 0.035, 0.37], [x + 0.068, 0.105, 0.50], c(PAD, shin));
        f.prism([x, 0.0, 0.07], [0.064, 0.07], [x, 0.0, 0.16], [0.06, 0.066], 8, c(BOOT, shin));
        f.cuboid([x - 0.062, -0.09, 0.0], [x + 0.062, 0.15, 0.085], c(BOOT, shin));
        f.cuboid([x - 0.058, 0.12, 0.0], [x + 0.058, 0.175, 0.06], c(COYOTE_DARK, shin));
    }
    // Hips, belt, torso.
    f.prism([0.0, 0.0, 0.83], [0.17, 0.11], [0.0, 0.0, 0.98], [0.163, 0.106], 10, c(UNIFORM, Part::Torso));
    f.prism([0.0, 0.0, 0.92], [0.176, 0.116], [0.0, 0.0, 0.965], [0.176, 0.116], 10, c(METAL, Part::Torso));
    f.prism([0.0, 0.0, 0.97], [0.155, 0.10], [0.0, 0.01, 1.41], [0.19, 0.115], 10, c(UNIFORM, Part::Torso));
    f.cuboid([0.17, -0.04, 0.84], [0.215, 0.05, 0.97], c(UNIFORM_DARK, Part::Torso));
    // Plate carrier: front and back plates, cummerbund, three magazine pouches, coyote shoulder straps.
    f.cuboid([-0.165, 0.10, 1.01], [0.165, 0.155, 1.36], c(VEST, Part::Torso));
    f.cuboid([-0.165, -0.155, 1.01], [0.165, -0.10, 1.36], c(VEST, Part::Torso));
    f.prism([0.0, 0.0, 1.0], [0.198, 0.13], [0.0, 0.0, 1.19], [0.205, 0.132], 10, c(VEST, Part::Torso));
    for x in [-0.12f32, -0.035, 0.05] {
        f.cuboid([x, 0.155, 1.03], [x + 0.07, 0.195, 1.17], c(METAL, Part::Torso));
    }
    for s in [-1.0f32, 1.0] {
        f.cuboid([s * 0.09 - 0.022, -0.125, 1.355], [s * 0.09 + 0.022, 0.125, 1.405], c(STRAP, Part::Torso));
        f.cuboid([s * 0.09 - 0.02, 0.15, 1.17], [s * 0.09 + 0.02, 0.16, 1.36], c(STRAP, Part::Torso));
    }
    // Pack with a pocket and a top flap.
    f.cuboid([-0.15, -0.31, 1.0], [0.15, -0.155, 1.38], c(PACK, Part::Torso));
    f.cuboid([-0.11, -0.345, 1.04], [0.11, -0.31, 1.22], c(COYOTE_DARK, Part::Torso));
    f.cuboid([-0.155, -0.315, 1.38], [0.155, -0.15, 1.42], c(COYOTE_DARK, Part::Torso));
    // Neck and head in a balaclava, goggles, helmet with rim and team band.
    f.prism([0.0, 0.0, 1.40], [0.05, 0.05], [0.0, 0.0, 1.52], [0.052, 0.052], 8, c(BALACLAVA, Part::Head));
    f.prism([0.0, 0.005, 1.50], [0.084, 0.095], [0.0, 0.0, 1.66], [0.08, 0.09], 10, c(BALACLAVA, Part::Head));
    f.prism([0.0, 0.0, 1.595], [0.09, 0.101], [0.0, 0.0, 1.645], [0.09, 0.101], 10, c(GOGGLE, Part::Head));
    f.cuboid([-0.072, 0.09, 1.598], [0.072, 0.112, 1.652], c(LENS, Part::Head));
    f.prism([0.0, 0.0, 1.615], [0.124, 0.136], [0.0, 0.0, 1.645], [0.124, 0.136], 10, c(HELMET, Part::Head));
    f.dome([0.0, 0.0, 1.645], [0.119, 0.13], 0.115, 10, 3, c(HELMET, Part::Head));
    f.prism([0.0, 0.0, 1.66], [0.12, 0.131], [0.0, 0.0, 1.69], [0.112, 0.123], 10, c(TEAM, Part::Head));
    // Arms: upper arm with a team patch, forearm with an elbow pad, gloved hand.
    for (s, arm, fore) in [(-1.0f32, Part::LeftArm, Part::LeftForearm), (1.0, Part::RightArm, Part::RightForearm)] {
        let x = s * 0.265;
        f.prism([x, 0.0, 1.14], [0.05, 0.054], [x, 0.0, 1.43], [0.062, 0.066], 8, c(UNIFORM, arm));
        f.cuboid([x + s * 0.05 - 0.012, -0.035, 1.27], [x + s * 0.05 + 0.012, 0.035, 1.35], c(TEAM, arm));
        f.prism([x, 0.0, 0.97], [0.044, 0.048], [x, 0.0, 1.16], [0.05, 0.054], 8, c(UNIFORM, fore));
        f.cuboid([x - 0.052, -0.075, 1.09], [x + 0.052, -0.03, 1.17], c(PAD, fore));
        f.cuboid([x - 0.036, -0.04, 0.89], [x + 0.036, 0.05, 0.98], c(GLOVE, fore));
    }
    // Rifle: stock, receiver, grip, magazine, handguard, barrel, optic.
    let r = |p: u8| code(p, Part::Rifle);
    f.cuboid([0.255, -0.20, 0.92], [0.295, 0.0, 0.995], r(METAL));
    f.cuboid([0.252, 0.0, 0.93], [0.298, 0.36, 1.01], r(METAL));
    f.cuboid([0.262, 0.02, 0.87], [0.288, 0.07, 0.93], r(VEST));
    f.cuboid([0.262, 0.12, 0.84], [0.288, 0.19, 0.93], r(VEST));
    f.cuboid([0.254, 0.36, 0.94], [0.296, 0.58, 1.0], r(COYOTE_DARK));
    f.cuboid([0.267, 0.58, 0.958], [0.283, 0.77, 0.976], r(METAL));
    f.cuboid([0.26, 0.08, 1.01], [0.29, 0.2, 1.055], r(VEST));
    f
}

/// The lighter figure for 70-250 m: same parts and proportions, a few boxes, the same colours.
pub fn figure_lo() -> MeshBuilder {
    use kit::*;
    let mut f = MeshBuilder::default();
    f.cuboid([-0.17, -0.09, 0.0], [-0.03, 0.09, 0.88], code(UNIFORM, Part::LeftThigh));
    f.cuboid([0.03, -0.09, 0.0], [0.17, 0.09, 0.88], code(UNIFORM, Part::RightThigh));
    f.cuboid([-0.19, -0.12, 0.85], [0.19, 0.12, 1.42], code(UNIFORM, Part::Torso));
    f.cuboid([-0.2, -0.15, 1.0], [0.2, 0.155, 1.36], code(VEST, Part::Torso));
    f.cuboid([-0.15, -0.31, 1.0], [0.15, -0.155, 1.38], code(PACK, Part::Torso));
    f.cuboid([-0.08, -0.09, 1.42], [0.08, 0.1, 1.62], code(BALACLAVA, Part::Head));
    f.cuboid([-0.12, -0.13, 1.6], [0.12, 0.14, 1.74], code(TEAM, Part::Head));
    f.cuboid([-0.31, -0.06, 1.12], [-0.22, 0.06, 1.42], code(UNIFORM, Part::LeftArm));
    f.cuboid([0.22, -0.06, 1.12], [0.31, 0.06, 1.42], code(UNIFORM, Part::RightArm));
    f.cuboid([-0.31, -0.06, 0.90], [-0.22, 0.06, 1.16], code(UNIFORM, Part::LeftForearm));
    f.cuboid([0.22, -0.06, 0.90], [0.31, 0.06, 1.16], code(UNIFORM, Part::RightForearm));
    f.cuboid([0.25, -0.1, 0.92], [0.30, 0.72, 1.0], code(METAL, Part::Rifle));
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
