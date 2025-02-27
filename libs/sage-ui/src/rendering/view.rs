use {
    bytemuck::{Pod, Zeroable},
    glam::UVec2,
    sage_wgpu::wgpu,
};

/// Represents the data that is sent to GPU shaders running in the UI pass.
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct View {
    /// The resolution of the surface.
    pub resolution: UVec2,
}

impl View {
    /// The size of a buffer storing a single [`View`].
    pub const BUFFER_SIZE: wgpu::BufferSize =
        wgpu::BufferSize::new(std::mem::size_of::<Self>() as _).unwrap();
}
