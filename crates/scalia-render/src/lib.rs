//! Scalia renderer: camera maths, meshes and shaders (tested natively), and the wgpu renderer.

pub mod camera;
pub mod mesh;
pub mod shaders;
pub mod gpu;

pub use wgpu::SurfaceTarget;
