//! Flat-shaded triangle lists: 3 floats of position and 1 colour code per vertex.
//! The colour code is `palette + 16 * part`; a negative code means "body, in the instance colour".
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

/// Colour code for a palette index on a part.
pub fn code(palette: u8, part: Part) -> f32 {
    (palette as u32 + 16 * part as u32) as f32
}

/// The part a colour code belongs to. Negative codes are the torso in the instance colour.
pub fn part_of(code: f32) -> Part {
    if code < 0.0 {
        return Part::Torso;
    }
    match (code / 16.0).floor() as u32 {
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

    pub fn bytes(&self) -> &[u8] {
        // Vertex is four f32 with no padding.
        unsafe { std::slice::from_raw_parts(self.vertices.as_ptr() as *const u8, self.vertices.len() * 16) }
    }
}

/// Low-poly soldier facing +y, 1.74 m to the top of the helmet. Thighs hang from the hips (0.88 m), shins with
/// boots from the knees (0.46 m), upper arms from the shoulders (1.40 m), forearms with hands from the elbows
/// (1.14 m), and the head nods around the neck (1.46 m). The torso is in the instance colour with webbing, belt,
/// pouches, canteen, pack and bedroll; the helmet has a rim and a strap; the rifle has stock, body, magazine,
/// barrel and sight and is gripped in the right hand.
pub fn figure() -> MeshBuilder {
    let mut f = MeshBuilder::default();
    let (dark, skin, helmet, gun, kit, boot, strap) = (3, 8, 10, 0, 13, 0, 13);
    for (side, thigh, shin) in [(-1.0f32, Part::LeftThigh, Part::LeftShin), (1.0, Part::RightThigh, Part::RightShin)] {
        let (x0, x1) = if side < 0.0 { (-0.17, -0.03) } else { (0.03, 0.17) };
        f.cuboid([x0, -0.09, 0.46], [x1, 0.09, 0.90], code(dark, thigh));
        f.cuboid([x0 + 0.01, -0.08, 0.07], [x1 - 0.01, 0.08, 0.48], code(dark, shin));
        f.cuboid([x0, -0.10, 0.0], [x1, 0.14, 0.08], code(boot, shin));
        // Gaiters above the boots.
        f.cuboid([x0 + 0.005, -0.085, 0.08], [x1 - 0.005, 0.085, 0.17], code(kit, shin));
    }
    // Torso, belt with two front pouches, canteen, webbing straps.
    f.cuboid([-0.22, -0.13, 0.92], [0.22, 0.13, 1.42], -1.0);
    f.cuboid([-0.23, -0.14, 0.86], [0.23, 0.14, 0.94], code(kit, Part::Torso));
    for x in [-0.15f32, 0.06] {
        f.cuboid([x, 0.13, 0.88], [x + 0.09, 0.18, 1.01], code(kit, Part::Torso));
    }
    f.cuboid([-0.26, -0.06, 0.80], [-0.21, 0.05, 0.94], code(dark, Part::Torso));
    for x in [-0.14f32, 0.10] {
        f.cuboid([x, -0.145, 0.94], [x + 0.04, 0.145, 1.42], code(strap, Part::Torso));
    }
    // Pack with a rolled bedroll on top.
    f.cuboid([-0.16, -0.25, 0.98], [0.16, -0.13, 1.36], code(kit, Part::Torso));
    f.cuboid([-0.19, -0.26, 1.36], [0.19, -0.14, 1.45], code(dark, Part::Torso));
    // Neck, head, helmet with rim and strap.
    f.cuboid([-0.06, -0.06, 1.40], [0.06, 0.07, 1.50], code(skin, Part::Head));
    f.cuboid([-0.09, -0.09, 1.48], [0.09, 0.11, 1.64], code(skin, Part::Head));
    f.cuboid([-0.12, -0.13, 1.58], [0.12, 0.14, 1.74], code(helmet, Part::Head));
    f.cuboid([-0.145, -0.155, 1.58], [0.145, 0.165, 1.615], code(helmet, Part::Head));
    f.cuboid([-0.095, 0.02, 1.49], [0.095, 0.05, 1.52], code(dark, Part::Head));
    // Upper arms with shoulder pads, forearms with hands.
    for (x0, x1, arm, fore) in [(-0.31f32, -0.23f32, Part::LeftArm, Part::LeftForearm), (0.23, 0.31, Part::RightArm, Part::RightForearm)] {
        f.cuboid([x0, -0.065, 1.12], [x1, 0.065, 1.42], code(dark, arm));
        f.cuboid([x0 - 0.01, -0.075, 1.34], [x1 + 0.01, 0.075, 1.44], code(kit, arm));
        f.cuboid([x0 + 0.005, -0.06, 0.98], [x1 - 0.005, 0.06, 1.16], code(dark, fore));
        f.cuboid([x0 + 0.01, -0.05, 0.90], [x1 - 0.01, 0.06, 0.98], code(skin, fore));
    }
    // Rifle: stock, body, magazine, barrel, rear sight.
    f.cuboid([0.25, -0.18, 0.92], [0.29, 0.02, 1.00], code(gun, Part::Rifle));
    f.cuboid([0.25, 0.02, 0.93], [0.30, 0.42, 1.01], code(gun, Part::Rifle));
    f.cuboid([0.26, 0.12, 0.85], [0.29, 0.20, 0.93], code(dark, Part::Rifle));
    f.cuboid([0.265, 0.42, 0.96], [0.285, 0.72, 0.98], code(gun, Part::Rifle));
    f.cuboid([0.268, 0.04, 1.01], [0.282, 0.09, 1.04], code(dark, Part::Rifle));
    f
}

/// The lighter figure for distant views: same proportions and parts, fewer pieces.
pub fn figure_lo() -> MeshBuilder {
    let mut f = MeshBuilder::default();
    f.cuboid([-0.17, -0.10, 0.0], [-0.03, 0.10, 0.88], code(3, Part::LeftThigh));
    f.cuboid([0.03, -0.10, 0.0], [0.17, 0.10, 0.88], code(3, Part::RightThigh));
    f.cuboid([-0.22, -0.13, 0.85], [0.22, 0.13, 1.45], -1.0);
    f.cuboid([-0.16, -0.24, 1.0], [0.16, -0.13, 1.35], code(13, Part::Torso));
    f.cuboid([-0.10, -0.10, 1.45], [0.10, 0.12, 1.62], code(8, Part::Head));
    f.cuboid([-0.12, -0.12, 1.56], [0.12, 0.14, 1.74], code(10, Part::Head));
    f.cuboid([-0.31, -0.07, 1.12], [-0.23, 0.07, 1.42], code(3, Part::LeftArm));
    f.cuboid([0.23, -0.07, 1.12], [0.31, 0.07, 1.42], code(3, Part::RightArm));
    f.cuboid([-0.31, -0.07, 0.90], [-0.23, 0.07, 1.16], code(3, Part::LeftForearm));
    f.cuboid([0.23, -0.07, 0.90], [0.31, 0.07, 1.16], code(3, Part::RightForearm));
    f.cuboid([0.25, -0.1, 0.92], [0.30, 0.72, 1.0], code(0, Part::Rifle));
    f
}

/// The far figure, for views where a soldier is a few pixels tall: a body in the instance colour and a helmet,
/// 24 triangles. It still leans and lies down with the poses.
pub fn figure_far() -> MeshBuilder {
    let mut f = MeshBuilder::default();
    f.cuboid([-0.2, -0.13, 0.0], [0.2, 0.13, 1.5], -1.0);
    f.cuboid([-0.12, -0.12, 1.5], [0.12, 0.14, 1.74], code(10, Part::Head));
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
