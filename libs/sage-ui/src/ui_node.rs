use {
    glam::{IVec2, UVec2},
    sage_core::{TypeUuid, Uuid, entities::Component},
};

/// A **component** that stores the computed metrics of a particular node.
#[derive(Default)]
pub struct UiNodeMetrics {
    /// The Z-index of the node.
    pub z_index: i32,
    /// The computed size of the node in physical pixels.
    pub size: UVec2,
    /// The computed position of the node, in physical pixels.
    pub position: IVec2,
    /// The offset of the baseline of the node from the top-left corner, in physical pixels.
    ///
    /// The baseline is the imaginary line that the last line displayed by the node is aligned to.
    pub baseline: IVec2,
}

unsafe impl TypeUuid for UiNodeMetrics {
    const UUID: Uuid = Uuid::from_u128(0xed163e1c38ff07d7e1c13b08e1ce6c9a);
}

impl Component for UiNodeMetrics {}
