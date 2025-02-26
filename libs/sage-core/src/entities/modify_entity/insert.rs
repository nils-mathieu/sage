use {
    super::ModifyEntity,
    crate::entities::{ArchetypeComponents, ArchetypeStorageRef, ComponentList, ComponentRegistry},
};

/// An implementation of [`ModifyEntity`] that inserts new components into an entity.
///
/// If the entity already has some of the components, this implementation will overwrite them.
pub struct Insert<C>(pub C);

unsafe impl<C> ModifyEntity for Insert<C>
where
    C: ComponentList,
{
    type Output = ();
    type ArchetypeComponents = Box<ArchetypeComponents>;

    fn modify_archetype(
        &self,
        registry: &mut ComponentRegistry,
        src: &ArchetypeComponents,
    ) -> Self::ArchetypeComponents {
        let mut vec = Vec::new();
        vec.extend_from_slice(src.as_uuids());
        self.0.register(registry, &mut |info| vec.push(info.uuid));
        ArchetypeComponents::from_unsorted_vec(vec)
    }

    unsafe fn modify_in_place(self, storage: ArchetypeStorageRef) -> Self::Output {
        self.0.write(&mut |uuid, src| unsafe {
            // SAFETY: The caller must provide the correct archetype.
            let (dst, info) = storage.get_raw_and_info(uuid).unwrap_unchecked();

            // Drop the old component.
            if let Some(drop_fn) = info.drop_fn {
                drop_fn(dst);
            }

            // Copy the new component over.
            std::ptr::copy_nonoverlapping(
                src.as_ptr::<u8>(),
                dst.as_ptr::<u8>(),
                info.layout.size(),
            );
        });
    }

    unsafe fn modify(self, src: ArchetypeStorageRef, dst: ArchetypeStorageRef) -> Self::Output {
        // Move the components from the source storage that are not already
        // covered by the component list.
        //
        // When already present, the component is dropped.
        for (uuid, info, data) in src.raw_components() {
            if self.0.has_component(uuid) {
                if let Some(drop_fn) = info.drop_fn {
                    unsafe { drop_fn(data) };
                }
            } else {
                // SAFETY: The caller must provide the correct archetype.
                let dst = unsafe { dst.get_raw(uuid).unwrap_unchecked() };

                // Copy the component over.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        data.as_ptr::<u8>(),
                        dst.as_ptr::<u8>(),
                        info.layout.size(),
                    );
                }
            }
        }

        self.0.write(&mut |uuid, src| unsafe {
            // SAFETY: The caller must provide the correct archetype.
            let (dst, info) = dst.get_raw_and_info(uuid).unwrap_unchecked();

            // Copy the component over.
            std::ptr::copy_nonoverlapping(
                src.as_ptr::<u8>(),
                dst.as_ptr::<u8>(),
                info.layout.size(),
            );
        });
    }
}
