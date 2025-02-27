use {
    crate::{
        UiNode,
        rendering::{RectInstance, UiPass},
    },
    glam::Vec4,
    sage_color::Srgba8,
    sage_core::{
        TypeUuid, Uuid,
        entities::Component,
        system::{Glob, Query},
    },
};

/// A **component** that draws a backgound behind the node.
#[derive(Debug, Clone, Copy)]
pub struct Background {
    /// The color of the background.
    pub color: Srgba8,
    /// The corner radius of the background.
    ///
    /// Order: top-left, top-right, bottom-right, bottom-left.
    pub corner_radius: [f32; 4],
    /// The thickness of the border.
    pub border_thickness: f32,
    /// The color of the border.
    pub border_color: Srgba8,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            color: Srgba8::WHITE,
            corner_radius: [0.0; 4],
            border_thickness: 0.0,
            border_color: Srgba8::TRANSPARENT,
        }
    }
}

unsafe impl TypeUuid for Background {
    const UUID: Uuid = Uuid::from_u128(0x2fc051b4ea1c6390f73df292e755647f);
}

impl Component for Background {}

/// A **system** that draws backgrounds behind the nodes.
pub(crate) fn draw_backgrounds(
    mut ui_pass: Glob<&mut UiPass>,
    query: Query<(&UiNode, &Background)>,
) {
    for (node, bg) in query.iter() {
        let has_background = !bg.color.is_transparent();
        let has_border = bg.border_thickness > 0.0 && !bg.border_color.is_transparent();

        if has_background {
            let mut r: Vec4 = bg.corner_radius.into();
            let mut position = node.position.as_vec2();
            let mut size = node.size.as_vec2();

            if has_border {
                let inset = bg.border_thickness * 0.5;
                position += inset;
                size -= bg.border_thickness;
                r = (r - inset).clamp(Vec4::ZERO, Vec4::splat((size * 0.5).min_element()));
            }

            ui_pass.add_rect_no_draw(RectInstance {
                position: position.round().as_ivec2(),
                size: size.round().as_uvec2(),
                color: bg.color,
                corner_radius: r,
                border_thickness: 0.0,
                _padding: [0; 2],
            });
        }

        if has_border {
            ui_pass.add_rect_no_draw(RectInstance {
                position: node.position,
                size: node.size,
                color: bg.border_color,
                corner_radius: Vec4::from(bg.corner_radius).clamp(
                    Vec4::ZERO,
                    Vec4::splat((node.size.as_vec2() * 0.5).min_element()),
                ),
                border_thickness: bg.border_thickness,
                _padding: [0; 2],
            });
        }

        ui_pass.submit_rects(node.z_index);
    }
}
