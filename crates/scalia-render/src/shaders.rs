//! WGSL sources; validated with naga in the test suite. Scene and shadow share `common.wgsl`
//! (uniforms and the walk animation) so shadows move with the figures.

pub const SCENE_WGSL: &str = concat!(include_str!("wgsl/common.wgsl"), include_str!("wgsl/scene.wgsl"));
pub const SHADOW_WGSL: &str = concat!(include_str!("wgsl/common.wgsl"), include_str!("wgsl/shadow.wgsl"));
pub const BLIT_WGSL: &str = include_str!("wgsl/blit.wgsl");
