//! WGSL sources. The scene shader gets the sun direction from `camera::sun` so shading and shadows agree.

use std::sync::LazyLock;

pub static SCENE_WGSL: LazyLock<String> = LazyLock::new(|| {
    let s = crate::camera::sun();
    include_str!("wgsl/scene.wgsl").replace("SUN_XYZ", &format!("{:.6}, {:.6}, {:.6}", s.x, s.y, s.z))
});
pub const SHADOW_WGSL: &str = include_str!("wgsl/shadow.wgsl");
pub const BLIT_WGSL: &str = include_str!("wgsl/blit.wgsl");
