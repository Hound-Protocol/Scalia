//! Bending the flat, east-west wrapping planet map into a globe as the camera zooms out.
//! The reference for the vertex shader's `bend` (`wgsl/common.wgsl`), which must match it.
//!
//! The map's x runs east and wraps every 2πR; its y runs north with the equator at y = 0, so latitude is y / R.
//! Bending wraps the map onto a sphere of radius R / t (plus each point's height), placed so the camera target
//! stays where it is: the target's surface point does not move, and the sphere's centre is R / t below it.

use glam::Vec3;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bend {
    /// Camera target on the flat map (metres).
    pub tx: f32,
    pub ty: f32,
    /// Planet radius in metres (map width / 2π).
    pub radius: f32,
    /// 0 is the flat map, 1 the full globe.
    pub t: f32,
    /// The target's own map y (metres north of the equator), which sets its latitude. With a floating origin
    /// `ty` is relative to the origin, so the latitude needs the absolute value.
    pub lat_y: f32,
}

impl Bend {
    pub const FLAT: Bend = Bend { tx: 0.0, ty: 0.0, radius: 1.0, t: 0.0, lat_y: 0.0 };
}

/// Where a flat-map point (x, y, height) is drawn.
pub fn bend(p: Vec3, b: &Bend) -> Vec3 {
    if b.t <= 0.0 {
        return p;
    }
    // The map lies on a sphere of radius R / t touching the target: almost flat at first, the planet itself at
    // t = 1. A true sphere at every step, so nothing folds the way a straight blend of positions would near the poles.
    let r = b.radius / b.t.min(1.0);
    globe_point(p.x - b.tx, p.y - b.ty, p.z, b.lat_y / r, r) + Vec3::new(b.tx, b.ty, 0.0)
}

/// A point `dx` metres east and `dy` metres north of the target (whose latitude is `lat0`), at height `h`, in the
/// frame of the target's surface point (x east, y north, z up). Written with half-angle sines so points near the
/// target keep full precision: no large numbers are subtracted.
pub fn globe_point(dx: f32, dy: f32, h: f32, lat0: f32, radius: f32) -> Vec3 {
    let rr = radius + h;
    let (lon, dlat) = (dx / radius, dy / radius);
    let lat = lat0 + dlat;
    let (sh_lon, sh_lat) = ((lon * 0.5).sin(), (dlat * 0.5).sin());
    let (cl, c0, s0) = (lat.cos(), lat0.cos(), lat0.sin());
    let vers_lon = 2.0 * sh_lon * sh_lon; // 1 - cos(lon)
    Vec3::new(
        rr * cl * lon.sin(),
        rr * (dlat.sin() + cl * s0 * vers_lon),
        h - rr * (2.0 * sh_lat * sh_lat + cl * c0 * vers_lon),
    )
}
