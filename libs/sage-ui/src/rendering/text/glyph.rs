use {
    bitflags::bitflags,
    bytemuck::{Pod, Zeroable},
    glam::{IVec2, UVec2},
    sage_color::Srgba8,
    sage_wgpu::wgpu,
};

bitflags! {
    /// The flags that are part of the [`GlyphInstance`] struct.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct GlyphInstanceFlags: u32 {
        /// Whether the glyph uses the mask texture.
        ///
        /// Otherwise, the color texture is used.
        const MASK_TEXTURE = 1 << 0;
    }
}

unsafe impl Zeroable for GlyphInstanceFlags {}
unsafe impl Pod for GlyphInstanceFlags {}

/// The instance of a glyph to draw.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GlyphInstance {
    /// The position of the glyph.
    pub position: IVec2,
    /// The size of the glyph.
    pub size: UVec2,
    /// The position of the glyph in the atlas.
    pub atlas_position: UVec2,
    /// The color of the glyph.
    pub color: Srgba8,
    /// Some flags.
    ///
    /// Bit 0: Whether the glyph uses the mask texture, or the color texture.
    pub flags: GlyphInstanceFlags,
}

impl GlyphInstance {
    /// The layout of an instance buffer containing [`GlyphInstance`]s.
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GlyphInstance>() as _,
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
                format: wgpu::VertexFormat::Uint32x2,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                offset: 24,
                format: wgpu::VertexFormat::Unorm8x4,
                shader_location: 3,
            },
            wgpu::VertexAttribute {
                offset: 28,
                format: wgpu::VertexFormat::Uint32,
                shader_location: 4,
            },
        ],
    };
}
