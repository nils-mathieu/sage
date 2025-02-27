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

macro_rules! impl_tuple {
    ($($name:ident)*) => {
        #[allow(unused_variables, non_snake_case)]
        impl<$($name,)*> ComponentSet for ($($name,)*)
        where
            $($name: ComponentList,)*
        {
            #[inline]
            fn has_component(&self, uuid: Uuid) -> bool {
                let ($($name,)*) = self;
                $($name.has_component(uuid) ||)* false
            }
        }

        #[allow(unused_variables, non_snake_case)]
        unsafe impl<$($name,)*> ComponentList for ($($name,)*)
        where
            $($name: ComponentList,)*
        {
            #[inline]
            fn register(
                &self,
                registry: &mut ComponentRegistry,
                callback: &mut impl FnMut(&'static ComponentInfo),
            ) {
                let ($($name,)*) = self;
                $($name.register(registry, callback);)*
            }

            #[inline]
            fn write(self, move_out: &mut impl FnMut(Uuid, OpaquePtr)) {
                let ($($name,)*) = self;
                $($name.write(move_out);)*
            }
        }
    };
}

impl_tuple!();
impl_tuple!(A);
impl_tuple!(A B);
impl_tuple!(A B C);
impl_tuple!(A B C D);
impl_tuple!(A B C D E);
impl_tuple!(A B C D E F);
impl_tuple!(A B C D E F G);
impl_tuple!(A B C D E F G H);
