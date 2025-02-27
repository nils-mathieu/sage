use {
    bytemuck::{Pod, Zeroable},
    glam::{IVec2, UVec2, Vec4},
    sage_color::Srgba8,
    sage_wgpu::wgpu,
};

/// A vertex that represents a rectangle's vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RectInstance {
    /// The position of the rectangle.
    pub position: IVec2,
    /// The size of the rectangle.
    pub size: UVec2,
    /// The corner radius of the rectangle.
    ///
    /// Order: top-left, top-right, bottom-right, bottom-left.
    pub corner_radius: Vec4,
    /// If non-zero, the rectangle is a border with the given thickness.
    pub border_thickness: f32,
    /// The color of the rectangle.
    pub color: Srgba8,

    pub _padding: [u32; 2],
}

impl RectInstance {
    /// The layout of an instance buffer containing [`RectInstance`]s.
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<RectInstance>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                format: wgpu::VertexFormat::Sint32x2,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                offset: 8,
                format: wgpu::VertexFormat::Uint32x2,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                offset: 16,
                format: wgpu::VertexFormat::Float32x4,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                offset: 32,
                format: wgpu::VertexFormat::Float32,
                shader_location: 3,
            },
            wgpu::VertexAttribute {
                offset: 36,
                format: wgpu::VertexFormat::Unorm8x4,
                shader_location: 4,
            },
        ],
    };
}
