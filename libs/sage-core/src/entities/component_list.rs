use {
    super::{Component, ComponentInfo, ComponentRegistry},
    crate::{OpaquePtr, Uuid},
    std::mem::ManuallyDrop,
};

/// A set of components.
pub trait ComponentSet {
    /// Returns whether the set contains a component with the specified UUID.
    fn has_component(&self, uuid: Uuid) -> bool;
}

impl<C> ComponentSet for C
where
    C: Component,
{
    #[inline]
    fn has_component(&self, uuid: Uuid) -> bool {
        uuid == C::UUID
    }
}

impl ComponentSet for () {
    #[inline]
    fn has_component(&self, _uuid: Uuid) -> bool {
        false
    }
}

/// A trait for types that can insert a collection of components into an entity.
///
/// # Safety
///
/// The [`ComponentSet`] implementation for the components in the list must return `true` for all
/// the UUIDs registered by [`register`](ComponentList::register) and `false` otherwise.
///
/// [`register`](ComponentList::register) must register distinct UUIDs that are controlled by the
/// implementation.
pub unsafe trait ComponentList: ComponentSet + 'static + Send + Sync {
    /// Calls the provided callback function with the components that are part
    /// of this [`ComponentList`].
    ///
    /// This function takes care of registering new components with the provided
    /// [`ComponentRegistry`].
    fn register(
        &self,
        registry: &mut ComponentRegistry,
        callback: &mut impl FnMut(&'static ComponentInfo),
    );

    /// Calls the `move_out` function on all components stored in this [`ComponentList`].
    ///
    /// # Remarks
    ///
    /// This function will assume that once it has called the function with a `*mut ()` pointer,
    /// that component is effectively moved out of the list. Specifically, it will assume it is
    /// no longer responsible for dropping it.
    fn write(self, move_out: &mut impl FnMut(Uuid, OpaquePtr));
}

unsafe impl<C> ComponentList for C
where
    C: Component,
{
    #[inline]
    fn register(
        &self,
        registry: &mut ComponentRegistry,
        callback: &mut impl FnMut(&'static ComponentInfo),
    ) {
        callback(registry.register::<C>());
    }

    #[inline]
    fn write(self, move_out: &mut impl FnMut(Uuid, OpaquePtr)) {
        let mut this = ManuallyDrop::new(self);
        move_out(C::UUID, OpaquePtr::from_mut::<C>(&mut this))
    }
}

unsafe impl ComponentList for () {
    fn register(
        &self,
        _registry: &mut ComponentRegistry,
        _callback: &mut impl FnMut(&'static ComponentInfo),
    ) {
    }
    fn write(self, _move_out: &mut impl FnMut(Uuid, OpaquePtr)) {}
}
