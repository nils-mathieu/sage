use {
    bytemuck::{Pod, Zeroable},
    glam::{Vec2, Vec4},
    sage_color::LinearSrgba,
    sage_wgpu::wgpu,
};

/// A vertex that represents a rectangle's vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct UiRectInstance {
    /// The position of the rectangle.
    pub position: Vec2,
    /// The size of the rectangle.
    pub size: Vec2,
    /// The color of the rectangle.
    pub color: LinearSrgba,
    /// The corner radius of the rectangle.
    ///
    /// Order: top-left, top-right, bottom-right, bottom-left.
    pub corner_radius: Vec4,
    /// If non-zero, the rectangle is a border with the given thickness.
    pub border_size: f32,

    /// The Z index of the rectangle.
    pub z_index: i32,

    pub _padding: [u32; 2],
}

impl UiRectInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<UiRectInstance>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                format: wgpu::VertexFormat::Float32x2,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                offset: 8,
                format: wgpu::VertexFormat::Float32x2,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                offset: 16,
                format: wgpu::VertexFormat::Float32x4,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                offset: 32,
                format: wgpu::VertexFormat::Float32x4,
                shader_location: 3,
            },
            wgpu::VertexAttribute {
                offset: 48,
                format: wgpu::VertexFormat::Float32,
                shader_location: 4,
            },
        ],
    };
}
