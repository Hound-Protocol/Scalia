//! Flat-shaded triangle lists: 3 floats of position and 1 colour code per vertex.
//! The colour code is `palette + 16 * part`; a negative code means "body, in the instance colour".
//! Parts are the limbs the vertex shader swings for the walk cycle.

/// Animated parts of a figure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Body = 0,
    LeftLeg = 1,
    RightLeg = 2,
    LeftArm = 3,
    RightArm = 4,
}

/// Colour code for a palette index on a part.
pub fn code(palette: u8, part: Part) -> f32 {
    (palette as u32 + 16 * part as u32) as f32
}

/// The part a colour code belongs to.
pub fn part_of(code: f32) -> Part {
    if code < 0.0 {
        return Part::Body;
    }
    match (code / 16.0).floor() as u32 {
        1 => Part::LeftLeg,
        2 => Part::RightLeg,
        3 => Part::LeftArm,
        4 => Part::RightArm,
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

/// Low-poly soldier facing +y, about 1.78 m tall: legs from the hips (0.88 m), arms from the shoulders
/// (1.42 m), a rifle on the right arm, a backpack. The torso takes the instance colour.
pub fn figure() -> MeshBuilder {
    let mut f = MeshBuilder::default();
    let (dark, skin, helmet, rifle, pack) = (3, 8, 10, 0, 13);
    f.cuboid([-0.17, -0.10, 0.0], [-0.03, 0.10, 0.88], code(dark, Part::LeftLeg));
    f.cuboid([0.03, -0.10, 0.0], [0.17, 0.10, 0.88], code(dark, Part::RightLeg));
    f.cuboid([-0.22, -0.13, 0.85], [0.22, 0.13, 1.45], -1.0);
    f.cuboid([-0.16, -0.24, 1.0], [0.16, -0.13, 1.35], code(pack, Part::Body));
    f.cuboid([-0.10, -0.10, 1.48], [0.10, 0.12, 1.70], code(skin, Part::Body));
    f.cuboid([-0.12, -0.12, 1.62], [0.12, 0.14, 1.78], code(helmet, Part::Body));
    f.cuboid([-0.31, -0.07, 0.90], [-0.23, 0.07, 1.42], code(dark, Part::LeftArm));
    f.cuboid([0.23, -0.07, 0.90], [0.31, 0.07, 1.42], code(dark, Part::RightArm));
    f.cuboid([0.27, 0.0, 1.0], [0.32, 0.75, 1.07], code(rifle, Part::RightArm));
    f
}
