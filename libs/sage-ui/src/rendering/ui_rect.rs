use {
    crate::Brush,
    bytemuck::{Pod, Zeroable},
    glam::Vec2,
    sage_color::LinearSrgba,
    sage_core::{TypeUuid, Uuid, entities::Component},
};

/// A **component** that ensures a particular UI node uses the common CSS-style background/border
/// styling.
#[derive(Debug, Clone)]
pub struct UiRect {
    /// The brush used to draw the background.
    pub background: Option<Brush>,
    /// The brush used to draw the border.
    ///
    /// The border is drawn inside of the node's bounds.
    pub border: Option<Brush>,
    /// The border radius.
    ///
    /// This is measured in physical pixels.
    ///
    /// Order: top-left, top-right, bottom-right, bottom-left.
    pub border_radius: [f32; 4],
    /// The border thickness.
    ///
    /// This is measured in physical pixels.
    ///
    /// Order: top, right, bottom, left.
    pub border_thickness: [f32; 4],
    /// The brush used to outline the node.
    ///
    /// It's drawn outside of the node's bounds.
    pub outline: Option<Brush>,
    /// The thickness of the outline.
    pub outline_thickness: f32,
    /// The offset of the outline from the node's bounds.
    pub outline_offset: f32,
}

unsafe impl TypeUuid for UiRect {
    const UUID: Uuid = Uuid::from_u128(0x319a192d00465aeef6e1d8c1ae1e11e7);
}

impl Component for UiRect {}

/// A vertex that represents a rectangle's vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct UiRectInstance {
    /// The position of the rectangle.
    pub position: Vec2,
    /// The size of the rectangle.
    pub size: Vec2,
    /// The background color of the rectangle.
    pub background_color: LinearSrgba,
    /// The border color of the rectangle.
    pub border_color: LinearSrgba,
    /// The outline color of the rectangle.
    pub outline_color: LinearSrgba,
    /// The border radius of the rectangle.
    ///
    /// Order: top-left, top-right, bottom-right, bottom-left.
    pub border_radius: [f32; 4],
    /// The border thickness of the rectangle.
    ///
    /// Order: top, right, bottom, left.
    pub border_thickness: [f32; 4],
    /// The thickness of the outline.
    pub outline_thickness: f32,
    /// The offset of the outline from the node's bounds.
    pub outline_offset: f32,

    /// Flags that control the rendering of the rectangle.
    ///
    /// Bit 0: Whether the rectangle has a background.
    /// Bit 1: Whether the rectangle has a border.
    /// Bit 2: Whether the rectangle has an outline.
    pub flags: u32,

    /// The Z-index of the rectangle.
    pub z_index: i32,
}
