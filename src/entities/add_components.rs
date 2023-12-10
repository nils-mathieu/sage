use alloc::boxed::Box;
use alloc::vec::Vec;

use super::entity_layout::{Components, EntityLayout, IntoEntityLayout};
use super::{Archetype, ComponentMeta, EditEntity, EntityPtr};

/// An implementation of [`EditEntity`] that adds components to an entity.
pub struct AddComponents<C>(pub C);

/// An implementation of [`IntoEntityLayout`] that adds components to an existing
/// archetype.
pub struct AddComponentsArchetype<'a, A> {
    archetype: &'a Archetype,
    layout: &'a EntityLayout,
    added: A,
}

unsafe impl<'a, A: IntoEntityLayout> IntoEntityLayout for AddComponentsArchetype<'a, A> {
    type ArchetypeStore<'b> = Box<Archetype>
    where
        Self: 'b;

    fn archetype(&self) -> Self::ArchetypeStore<'_> {
        let added = self.added.archetype();
        let added = added.as_ref();

        let mut new = Vec::with_capacity(self.archetype.ids().len() + added.ids().len());

        // We know this won't overflow because we allocated enough capacity for the worst case.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.archetype.ids().as_ptr(),
                new.as_mut_ptr(),
                self.archetype.ids().len(),
            );
        }

        for &added_id in added.ids() {
            if !self.layout.has_component(added_id) {
                // We know that this won't ever overflow because we allocated enough
                // capacity for the worst case.
                unsafe {
                    new.as_mut_ptr().add(new.len()).write(added_id);
                    new.set_len(new.len() + 1);
                }
            }
        }

        new.sort_unstable();

        // SAFETY:
        //  We added the IDs in ascending order, so we know that the resulting archetype
        //  is valid.
        unsafe { Archetype::new_boxed(new.into_boxed_slice()) }
    }

    fn into_layout(self) -> EntityLayout {
        let layout = self.added.into_layout();

        let mut new: Vec<ComponentMeta> =
            Vec::with_capacity(self.layout.component_count() + layout.component_count());

        // We know this won't overflow because we allocated enough capacity for the worst case.
        for field in self.layout.components() {
            unsafe {
                new.as_mut_ptr().add(new.len()).write(field.meta);
                new.set_len(new.len() + 1);
            }
        }

        for field in layout.components() {
            if !self.layout.has_component(field.meta.id()) {
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

unsafe impl<C: Components> EditEntity for AddComponents<C> {
    type Archetype<'a> = AddComponentsArchetype<'a, C::Archetype<'a>>
    where
        Self: 'a;

    #[inline]
    unsafe fn new_archetype<'a>(
        &'a self,
        archetype: &'a Archetype,
        layout: &'a EntityLayout,
    ) -> Self::Archetype<'a> {
        AddComponentsArchetype {
            archetype,
            layout,
            added: self.0.archetype(),
        }
    }

    type Output = ();

    unsafe fn edit_in_place(self, entity: EntityPtr) -> Self::Output {
        entity.write(self.0);
    }

    unsafe fn edit(self, old: EntityPtr, new: EntityPtr) -> Self::Output {
        // Copy the components from the old entity to the new one.
        for (data, meta) in old.components() {
            let (dst, _) = new.get_field_unchecked(meta.id());
            core::ptr::copy_nonoverlapping(data, dst, meta.layout().size());
        }

        self.0.write_components(|id, ptr| {
            if let Some((ptr, field)) = old.get_field(id) {
                // If the component has already been copied from the old entity, then we have to
                // drop it.
                field.meta.drop_in_place(ptr);
            }

            let (dst, field) = new.get_field_unchecked(id);
            core::ptr::copy_nonoverlapping(ptr, dst, field.meta.layout().size());
        });
    }
}
