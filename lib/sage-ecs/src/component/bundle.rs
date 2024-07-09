use super::{ComponentId, Registry};

/// Describes how to insert the components of a bundle into an entity.
///
/// # Safety
///
/// When the function `dst` in [`insert`] returns a non-null pointer, it must be initialized
/// with a valid instance of the component type.
///
/// [`insert`]: InsertBundle::insert
pub unsafe trait InsertBundle {
    /// Inserts the components that are part of the provided [`SparseSet`] into the entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that when `dst` returns a non-null pointer, that pointers references
    /// a valid memory location for an instance of the component to be initialized.
    unsafe fn insert(self, dst: impl FnMut(ComponentId) -> *mut u8);
}

unsafe impl InsertBundle for () {
    unsafe fn insert(self, _dst: impl FnMut(ComponentId) -> *mut u8) {}
}

/// Rust types that can be used as a component bundle.
///
/// # Safety
///
/// Implementors of this trait must ensure that the [`register`] method properly registers the
/// components that are part of the bundle and return their IDs. The IDs must be unique and
/// consistent across different invocations of the method.
#[cfg(feature = "rust-components")]
pub unsafe trait Bundle: 'static + InsertBundle {
    /// Registers the components and the bundle that are associated with this type.
    fn register_components(registry: &mut Registry) -> Box<[ComponentId]>;
}
