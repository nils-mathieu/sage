//! Atomic fragment of an entity.
//!
//! # Rust types
//!
//! In [`sage-ecs`](crate), components are not necessarily Rust types. Instead, they are just a
//! collection of bytes with an associated drop function. This allows components to come from any
//! language or runtime.

use core::alloc::Layout;

use alloc::borrow::Cow;

/// A function that is responsible for dropping a component instance.
///
/// For regular Rust types, this function is generally just `std::ptr::drop_in_place`. But external
/// components may require a custom drop function.
///
/// # Safety
///
/// After this function has been called on a memory location owning a component instance, the
/// memory location must be considered *uninitialized* and may not be accessed again.
pub type DropFn = unsafe fn(*mut u8);

/// Stores information about a component type.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// The name of the component type. This is mainly used for debugging purposes.
    pub name: Cow<'static, str>,
    /// A function that must be called in order to drop a component instance.
    ///
    /// This function is responsible for freeing any resources that the component instance may
    /// own.
    ///
    /// If the component does not need to be dropped, this field is `None`.
    pub drop_fn: Option<DropFn>,
    /// The memory layout of the component type. A continuous block of bytes that fits such layout
    /// is suitable for storing an instance of this component.
    pub layout: Layout,
}
