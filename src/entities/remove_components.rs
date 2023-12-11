use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::{
    Archetype, Component, ComponentId, ComponentMeta, EditEntity, EntityLayout, EntityPtr,
    IntoEntityLayout,
};

/// A set of components.
pub trait ComponentSet {
    /// Returns whether the provided [`TypeId`] is part of this set.
    fn contains(&self, id: ComponentId) -> bool;
}

impl ComponentSet for ComponentId {
    #[inline(always)]
    fn contains(&self, id: ComponentId) -> bool {
        *self == id
    }
}

/// A static component set.
pub struct StaticComponentSet<C: ?Sized>(PhantomData<C>);

impl<C> Default for StaticComponentSet<C> {
    #[inline(always)]
    fn default() -> Self {
        Self(PhantomData)
    }
}

macro_rules! impl_for_tuple {
    ($($ty:ident),*) => {
        impl<$($ty: Component,)*> ComponentSet for StaticComponentSet<($($ty,)*)> {
            #[inline(always)]
            #[allow(unused_variables)]
            fn contains(&self, id: ComponentId) -> bool {
                $(
                    id == ComponentId::of::<$ty>() ||
                )* false
            }
        }
    };
}

impl_for_tuple!();
impl_for_tuple!(A);
impl_for_tuple!(A, B);
impl_for_tuple!(A, B, C);
impl_for_tuple!(A, B, C, D);
impl_for_tuple!(A, B, C, D, E);
impl_for_tuple!(A, B, C, D, E, F);

impl<T: Component> ComponentSet for StaticComponentSet<T> {
    #[inline]
    fn contains(&self, id: ComponentId) -> bool {
        id == ComponentId::of::<T>()
    }
}

/// An implementation of [`EditEntity`] that removes components from an entity.
pub struct RemoveComponents<'a, S>(pub &'a S);

/// An implementation of [`IntoEntityLayout`] that removes components from an existing
/// archetype.
pub struct RemoveComponentsArchetype<'a, S> {
    archetype: &'a Archetype,
    layout: &'a EntityLayout,
    set: &'a S,
}

unsafe impl<'a, S: ComponentSet> IntoEntityLayout for RemoveComponentsArchetype<'a, S> {
    type ArchetypeStore<'b> = Box<Archetype>
    where
        Self: 'b;

    #[inline]
    fn archetype(&self) -> Self::ArchetypeStore<'_> {
        // NOTE:
        //  Not sure whether it's better to allocate a new vector and copy the IDs like we're doing
        //  right now, or if it's better to count the number of IDs that we're going to keep and
        //  allocate a vector of that size.
        //  I think this really depends on the `set` that we're given.

        let mut new: Vec<ComponentId> = Vec::with_capacity(self.archetype.ids().len());

        for &id in self.archetype.ids().iter() {
            if !self.set.contains(id) {
                // We know that this won't ever overflow because we allocated enough
                // capacity for the worst case.
                unsafe {
                    new.as_mut_ptr().add(new.len()).write(id);
                    new.set_len(new.len() + 1);
                }
            }
        }

        // SAFETY:
        //  We added the IDs in ascending order, so we know that the resulting archetype
        //  is valid.
        unsafe { Archetype::new_boxed(new.into_boxed_slice()) }
    }

    fn into_layout(self) -> EntityLayout {
        let mut new: Vec<ComponentMeta> = Vec::with_capacity(self.archetype.ids().len());

        for field in self.layout.components() {
            if !self.set.contains(field.meta.id()) {
                // We know that this won't ever overflow because we allocated enough
                // capacity for the worst case.
                unsafe {
                    new.as_mut_ptr().add(new.len()).write(field.meta);
                    new.set_len(new.len() + 1);
                }
            }
        }

        new.sort_unstable_by_key(|field| field.layout().align());

        // SAFETY:
        //  We made sure to properly sort the fields by their alignment.
        unsafe { EntityLayout::new_unchecked(new.into_iter()) }
    }
}

unsafe impl<'s, S: ComponentSet> EditEntity for RemoveComponents<'s, S> {
    type Archetype<'a> = RemoveComponentsArchetype<'a, S>
    where
        Self: 'a;

    #[inline(always)]
    unsafe fn new_archetype<'a>(
        &'a self,
        archetype: &'a Archetype,
        layout: &'a EntityLayout,
    ) -> Self::Archetype<'a> {
        RemoveComponentsArchetype {
            archetype,
            layout,
            set: self.0,
        }
    }

    type Output = ();

    unsafe fn edit(self, old: EntityPtr, new: EntityPtr) -> Self::Output {
        for (data, meta) in old.components() {
            if !self.0.contains(meta.id()) {
                // The field is not part of the set, meaning that it's kept.
                // We need to copy it to the other entity.
                let (dst, _) = new.get_field_unchecked(meta.id());
                core::ptr::copy_nonoverlapping(data, dst, meta.layout().size());
            } else {
                // Otherwise, we need to drop it.
                meta.drop_in_place(data);
            }
        }
    }

    unsafe fn edit_in_place(self, _entity: EntityPtr) -> Self::Output {
        // Nothing to do.
        // If removing a component from the set does not change the archetype, then there isn't
        // actually anything to remove.
    }
}
