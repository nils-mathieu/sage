use crate::{OpaquePtr, Uuid, entities::ArchetypeComponents};

/// A trait for types that can insert a collection of components into an entity.
///
/// # Safety
///
/// The implementor must make sure that implemented methods are coherent with one another.
pub unsafe trait ComponentList {
    /// The type that stores the components of an archetype.
    ///
    /// This is the return type of [`archetype_components`].
    ///
    /// [`archetype_components`]: ComponentList::archetype_components
    type ArchetypeComponents: AsRef<ArchetypeComponents> + Into<Box<ArchetypeComponents>>;

    /// Returns the components that this list contains.
    fn archetype_components(&self) -> Self::ArchetypeComponents;

    /// Calls the `move_out` function on all components stored in this [`ComponentList`].
    ///
    /// # Remarks
    ///
    /// This function will assume that once it has called the function with a `*mut ()` pointer,
    /// that component is effectively moved out of the list. Specifically, it will assume it is
    /// no longer responsible for dropping it.
    fn write(self, move_out: impl FnMut(Uuid, OpaquePtr));
}
