//! What a fight looks like for a moment: muzzle flashes (with a light that throws a warm pool around the shooter,
//! far brighter by night), tracers streaking to where the round went, puffs where it landed, and grenade blasts.
//! Positions are absolute metres (x, y, z); drawing takes the floating origin.
use scalia_render::gpu::PointLight;
use scalia_render::mesh::{code, MeshBuilder, Part};

/// How long each effect lasts (ms).
const FLASH_MS: f64 = 55.0;
const FLASH_LIGHT_MS: f64 = 70.0;
const PUFF_MS: f64 = 450.0;
const BLAST_MS: f64 = 180.0;
const BLAST_LIGHT_MS: f64 = 300.0;
const DUST_MS: f64 = 1_600.0;
/// Rounds fly at about 900 m/s; a tracer is drawn as a 3 m streak.
const ROUND_M_PER_MS: f64 = 0.9;
const TRACER_M: f64 = 3.0;
/// Glowing colours (palette entries drawn unlit): flash yellow-white, tracer orange.
const FLASH_GLOW: u8 = 14;
const TRACER_GLOW: u8 = 5;
/// Lit colours: dust, and the dark red of a hit.
const DUST: u8 = 13;
const BLOOD: u8 = 9;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Shot {
    at: f64,
    muzzle: [f64; 3],
    impact: [f64; 3],
    hit: bool,
    tracer: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Blast {
    at: f64,
    pos: [f64; 3],
}

#[derive(Default)]
pub struct Effects {
    shots: Vec<Shot>,
    blasts: Vec<Blast>,
    /// This frame's smoke clouds: (centre on the ground, age in ms, which cloud).
    smoke: Vec<([f64; 3], f64, u64)>,
}

/// A smoke cloud's life (the simulation's SMOKE_BUILD_MS and SMOKE_MS), its radius, and its colours.
const SMOKE_BUILD_MS: f64 = scalia_ai::battle::SMOKE_BUILD_MS as f64;
const SMOKE_MS: f64 = scalia_ai::battle::SMOKE_MS as f64;
const SMOKE_R_M: f64 = scalia_ai::battle::SMOKE_R_MM as f64 / 1000.0;
const SMOKE_GREY: u8 = 2;
const SMOKE_PALE: u8 = 15;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn len(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

impl Effects {
    /// A round fired at viewer time `at` (ms) from `muzzle` to `impact`; `tracer` draws its streak.
    pub fn shot(&mut self, at: f64, muzzle: [f64; 3], impact: [f64; 3], hit: bool, tracer: bool) {
        self.shots.push(Shot { at, muzzle, impact, hit, tracer });
    }

    pub fn blast(&mut self, at: f64, pos: [f64; 3]) {
        self.blasts.push(Blast { at, pos });
    }

    /// The smoke clouds standing this frame (replacing the last frame's).
    pub fn set_smoke(&mut self, clouds: Vec<([f64; 3], f64, u64)>) {
        self.smoke = clouds;
    }

    /// Drops what has finished by `now`.
    pub fn prune(&mut self, now: f64) {
        self.shots.retain(|s| now - s.at < PUFF_MS + len(sub(s.impact, s.muzzle)) / ROUND_M_PER_MS);
        self.blasts.retain(|b| now - b.at < DUST_MS);
    }

    pub fn is_empty(&self) -> bool {
        self.shots.is_empty() && self.blasts.is_empty() && self.smoke.is_empty()
    }

    /// The lights at `now`, newest first (the renderer takes the first MAX_LIGHTS), relative to `origin`.
    pub fn lights(&self, now: f64, origin: (f64, f64)) -> Vec<PointLight> {
        let rel = |p: [f64; 3]| [(p[0] - origin.0) as f32, (p[1] - origin.1) as f32, p[2] as f32];
        let mut out: Vec<(f64, PointLight)> = Vec::new();
        for b in &self.blasts {
            let age = now - b.at;
            if (0.0..BLAST_LIGHT_MS).contains(&age) {
                let k = 1.0 - age / BLAST_LIGHT_MS;
                let mut p = rel(b.pos);
                p[2] += 1.5;
                out.push((b.at + 1e6, PointLight { pos: p, radius: 16.0, color: [1.0, 0.62, 0.3], intensity: (3.2 * k * k) as f32 }));
            }
        }
        for s in &self.shots {
            let age = now - s.at;
            if (0.0..FLASH_LIGHT_MS).contains(&age) {
                let k = 1.0 - age / FLASH_LIGHT_MS;
                out.push((s.at, PointLight { pos: rel(s.muzzle), radius: 7.0, color: [1.0, 0.78, 0.45], intensity: (1.8 * k) as f32 }));
            }
        }
        // Blasts first, then the newest flashes.
        out.sort_by(|a, b| b.0.total_cmp(&a.0));
        out.into_iter().map(|l| l.1).collect()
    }

    /// The geometry at `now`, relative to `origin`.
    pub fn mesh(&self, now: f64, origin: (f64, f64)) -> MeshBuilder {
        let rel = |p: [f64; 3]| [(p[0] - origin.0) as f32, (p[1] - origin.1) as f32, p[2] as f32];
        let mut m = MeshBuilder::default();
        for s in &self.shots {
            let age = now - s.at;
            if age < 0.0 {
                continue;
            }
            let path = sub(s.impact, s.muzzle);
            let dist = len(path).max(0.01);
            if age < FLASH_MS {
                // A star of fire at the muzzle: a bright core and a cone thrown forward.
                let k = (1.0 - age / FLASH_MS) as f32;
                let tip = lerp(s.muzzle, s.impact, (0.55 / dist).min(1.0));
                m.diamond(rel(s.muzzle), 0.16 * k + 0.05, code(FLASH_GLOW, Part::Glow));
                m.prism(rel(s.muzzle), [0.09 * k, 0.09 * k], rel(tip), [0.01, 0.01], 5, code(FLASH_GLOW, Part::Glow));
            }
            let flight = dist / ROUND_M_PER_MS;
            if s.tracer && age < flight {
                let head = (age * ROUND_M_PER_MS / dist).min(1.0);
                let tail = ((age * ROUND_M_PER_MS - TRACER_M) / dist).max(0.0);
                m.prism(rel(lerp(s.muzzle, s.impact, tail)), [0.035, 0.035], rel(lerp(s.muzzle, s.impact, head)), [0.045, 0.045], 4, code(TRACER_GLOW, Part::Glow));
            }
            let since = age - flight;
            if (0.0..PUFF_MS).contains(&since) {
                // Where it landed: a little spray of red on a hit, a kick of dust on a miss.
                let t = (since / PUFF_MS) as f32;
                let r = 0.08 + 0.35 * t;
                let mut p = rel(s.impact);
                p[2] += 0.25 * t;
                let c = if s.hit { code(BLOOD, Part::Body) } else { code(DUST, Part::Body) };
                let r = if s.hit { r * 0.6 } else { r };
                m.cuboid([p[0] - r, p[1] - r, p[2] - r * 0.6], [p[0] + r, p[1] + r, p[2] + r * 0.6], c);
            }
        }
        for &(c, age, id) in &self.smoke {
            // Billowing out over the first seconds, then thinning: shrinking and sinking in its last eight.
            let grow = (age / SMOKE_BUILD_MS).clamp(0.0, 1.0).sqrt();
            let thin = ((SMOKE_MS - age) / 8_000.0).clamp(0.0, 1.0);
            let k = (grow * (0.35 + 0.65 * thin)) as f32;
            if k <= 0.01 {
                continue;
            }
            let base = rel(c);
            // A heap of puffs around the centre (fixed per cloud), slowly turning.
            for n in 0..7u64 {
                let h = scalia_core::hash::mix64(id ^ n.wrapping_mul(0x9e37_79b9));
                let a = (h % 6283) as f64 / 1000.0 + age / 40_000.0;
                let off = if n == 0 { 0.0 } else { SMOKE_R_M * (0.35 + ((h >> 16) % 450) as f64 / 1000.0) };
                let r = SMOKE_R_M as f32 * (0.45 + ((h >> 32) % 250) as f32 / 1000.0) * k;
                let p = [base[0] + (a.cos() * off) as f32 * k, base[1] + (a.sin() * off) as f32 * k, base[2]];
                let colour = if n % 3 == 0 { SMOKE_PALE } else { SMOKE_GREY };
                m.dome(p, [r, r], r * (1.1 + 0.4 * thin as f32), 9, 4, code(colour, Part::Body));
            }
        }
        for b in &self.blasts {
            let age = now - b.at;
            if age < 0.0 {
                continue;
            }
            let p = rel(b.pos);
            if age < BLAST_MS {
                let k = (age / BLAST_MS) as f32;
                let r = 0.6 + 2.6 * k;
                m.dome(p, [r, r], r * 0.9, 10, 4, code(FLASH_GLOW, Part::Glow));
            }
            if age < DUST_MS {
                let k = (age / DUST_MS) as f32;
                let r = 1.2 + 4.0 * k.sqrt();
                let h = (2.8 * (1.0 - k)).max(0.2);
                m.dome(p, [r, r], h, 10, 3, code(DUST, Part::Body));
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_lights_up_briefly_and_its_tracer_flies_to_the_target() {
        let mut fx = Effects::default();
        fx.shot(1_000.0, [100.0, 100.0, 1.4], [100.0, 190.0, 1.2], false, true);
        assert!(fx.lights(990.0, (0.0, 0.0)).is_empty(), "not before it is fired");
        let l = fx.lights(1_010.0, (0.0, 0.0));
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].pos, [100.0, 100.0, 1.4]);
        assert!(fx.lights(1_100.0, (0.0, 0.0)).is_empty(), "the flash is gone in a tenth of a second");
        // 60 ms into a 100 ms flight (the flash is over): only the tracer, 51-54 m along the path.
        let mid = fx.mesh(1_060.0, (0.0, 0.0));
        assert!(!mid.vertices.is_empty());
        let ys: Vec<f32> = mid.vertices.iter().map(|v| v.pos[1]).collect();
        let (lo, hi) = (ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max));
        assert!(lo > 150.5 && hi < 154.5, "streak at 151-154 m: {lo}..{hi}");
        // After the puff, nothing is left.
        fx.prune(2_000.0);
        assert!(fx.is_empty());
    }

    #[test]
    fn blasts_come_first_and_lights_are_newest_first() {
        let mut fx = Effects::default();
        fx.shot(1_000.0, [0.0, 0.0, 1.4], [0.0, 30.0, 1.0], true, false);
        fx.shot(1_020.0, [5.0, 0.0, 1.4], [5.0, 30.0, 1.0], true, false);
        fx.blast(1_010.0, [10.0, 10.0, 0.0]);
        let l = fx.lights(1_030.0, (0.0, 0.0));
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].radius, 16.0, "the blast");
        assert_eq!(l[1].pos[0], 5.0, "then the newer flash");
    }
}
