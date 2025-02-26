use super::{ArchetypeComponents, ArchetypeStorageRef, ComponentRegistry};

mod insert;
pub use self::insert::*;

/// Represents an operation that can modify the set of components of an entity.
///
/// # Safety
///
/// The [`ModifyEntity::modify`] method must push the created entity to the end of the destination
/// archetype.
pub unsafe trait ModifyEntity {
    /// The eventual output of the operation.
    type Output;

    /// The return-type of the [`ModifyEntity::archetype`] method.
    type ArchetypeComponents: AsRef<ArchetypeComponents> + Into<Box<ArchetypeComponents>>;

    /// Given an entity's archetype components, returns the new set of components
    /// of the entity after applying the modification.
    ///
    /// This function takes care of registering any new components to the component registry.
    fn modify_archetype(
        &self,
        registery: &mut ComponentRegistry,
        src: &ArchetypeComponents,
    ) -> Self::ArchetypeComponents;

    /// Modifies the entity's components in-place.
    ///
    /// # Safety
    ///
    /// The caller must ensure that given the storage's archetype components, when applying the
    /// [`ModifyEntity::archetype`] method, the output set of components is equal to the input set
    /// of components.
    unsafe fn modify_in_place(self, dst: ArchetypeStorageRef) -> Self::Output;

    /// Modifies the entity's components out-of-place.
    ///
    /// This function will move the `src` component out, and will initialize `dst`, overwriting
    /// the previous value without touching it.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the source and destination storages are coherent with what
    ///   the [`ModifyEntity::archetype`] method has returned. The set of components in
    ///   `dst_archetype` must be equal to the set of components in `src_archetype` after
    ///   applying the [`ModifyEntity::archetype`] method.
    ///
    /// - `src` must follow the archetype that was passed to the [`ModifyEntity::archetype`]
    ///   to get `dst`.
    unsafe fn modify(self, src: ArchetypeStorageRef, dst: ArchetypeStorageRef) -> Self::Output;
}
