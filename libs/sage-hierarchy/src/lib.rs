//! A parent-children hierarchy system for the Sage game engine.

use sage_core::{
    TypeUuid, Uuid,
    entities::{Component, EntityId},
};

/// The **component** responsible for storing the parent of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub EntityId);

unsafe impl TypeUuid for Parent {
    const UUID: Uuid = Uuid::from_u128(0x139530b2d77a1ca1ebf060d28b0cd936);
}

impl Component for Parent {}

/// The **component** responsible for storing the children of an entity.
#[derive(Debug, Clone)]
pub struct Children(pub Vec<EntityId>);

unsafe impl TypeUuid for Children {
    const UUID: Uuid = Uuid::from_u128(0x4df7aa8a106b12db5d0e3d439760491a);
}

impl Component for Children {}
