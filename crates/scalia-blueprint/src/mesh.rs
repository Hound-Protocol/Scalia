//! Integer mesh generation from blueprint primitives. Positions are millimetres.
//! Frame matches `validate`: boxes and wedges use their minimum corner, cylinders
//! the centre of their base. A wedge slopes from full height at +y to zero at -y.

use crate::types::{Blueprint, Prim};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Mesh {
    pub vertices: Vec<[i32; 3]>,
    pub indices: Vec<u32>,
    /// One palette index per triangle.
    pub colors: Vec<u8>,
}

impl Mesh {
    fn tri(&mut self, base: u32, a: u32, b: u32, c: u32, color: u8) {
        self.indices.extend_from_slice(&[base + a, base + b, base + c]);
        self.colors.push(color);
    }
}

/// Cosine of multiples of 45 degrees, times 1000: 1000, 707, 0, -707, -1000, -707, 0, 707.
const COS: [i64; 8] = [1000, 707, 0, -707, -1000, -707, 0, 707];
const SIN: [i64; 8] = [0, 707, 1000, 707, 0, -707, -1000, -707];

fn box_corners(x: i32, y: i32, z: i32, w: u32, d: u32, h: u32) -> [[i32; 3]; 8] {
    let (x1, y1, z1) = (x + w as i32, y + d as i32, z + h as i32);
    [
        [x, y, z], [x1, y, z], [x, y1, z], [x1, y1, z],
        [x, y, z1], [x1, y, z1], [x, y1, z1], [x1, y1, z1],
    ]
}

pub fn mesh(b: &Blueprint) -> Mesh {
    let mut m = Mesh::default();
    for p in &b.model {
        let base = m.vertices.len() as u32;
        match *p {
            Prim::Box { x, y, z, w, d, h, color } => {
                m.vertices.extend_from_slice(&box_corners(x, y, z, w, d, h));
                // corner index = x + 2*y + 4*z
                for t in [
                    [0, 2, 1], [1, 2, 3], // bottom
                    [4, 5, 6], [5, 7, 6], // top
                    [0, 1, 4], [1, 5, 4], // front (-y)
                    [2, 6, 3], [3, 6, 7], // back (+y)
                    [0, 4, 2], [2, 4, 6], // left (-x)
                    [1, 3, 5], [3, 7, 5], // right (+x)
                ] {
                    m.tri(base, t[0], t[1], t[2], color);
                }
            }
            Prim::Wedge { x, y, z, w, d, h, color } => {
                let (x1, y1) = (x + w as i32, y + d as i32);
                m.vertices.extend_from_slice(&[
                    [x, y, z], [x1, y, z], [x, y1, z], [x1, y1, z], [x, y1, z + h as i32], [x1, y1, z + h as i32],
                ]);
                for t in [
                    [0, 2, 1], [1, 2, 3], // bottom
                    [2, 4, 3], [3, 4, 5], // back
                    [0, 1, 5], [0, 5, 4], // slope
                    [0, 4, 2],            // left
                    [1, 3, 5],            // right
                ] {
                    m.tri(base, t[0], t[1], t[2], color);
                }
            }
            Prim::Cylinder { x, y, z, radius, h, color } => {
                let r = radius as i64;
                for level in 0..2i32 {
                    let zz = z + level * h as i32;
                    for k in 0..8 {
                        let dx = (r * COS[k]).div_euclid(1000) as i32;
                        let dy = (r * SIN[k]).div_euclid(1000) as i32;
                        m.vertices.push([x + dx, y + dy, zz]);
                    }
                }
                for k in 0..8u32 {
                    let n = (k + 1) % 8;
                    m.tri(base, k, n, 8 + k, color);
                    m.tri(base, n, 8 + n, 8 + k, color);
                }
                for k in 1..7u32 {
                    m.tri(base, 0, k + 1, k, color); // bottom cap fan
                    m.tri(base, 8, 8 + k, 8 + k + 1, color); // top cap fan
                }
            }
        }
    }
    m
}

/// FNV-1a over the little-endian bytes of vertices, indices and colours.
pub fn mesh_hash(m: &Mesh) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for v in &m.vertices {
        for c in v {
            eat(&c.to_le_bytes());
        }
    }
    for i in &m.indices {
        eat(&i.to_le_bytes());
    }
    eat(&m.colors);
    h
}
