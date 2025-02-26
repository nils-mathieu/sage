use crate::entities::{
    ArchetypeComponents, ArchetypeStorage, ComponentList, EntityId, EntityIdAllocator, EntityIndex,
    EntityMut, EntityRef,
};

/// An identifier for an archetype stored in the [`World`].
pub type ArchetypeId = usize;

/// The location of an entity stored in the [`World`].
///
/// This is a bit like [`EntityId`], except this is not stable across component modifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EntityLocation {
    /// The index of the entity's column.
    pub archetype: ArchetypeId,
    /// The index of the entity within its archetype storage.
    pub index: usize,
}

/// The map type used by the [`Entities`] collection.
type Map<K, V> = hashbrown::HashMap<K, V, foldhash::fast::FixedState>;

/// A collection of entities that are stored efficiently in memory and grouped using the
/// components that make them up.
#[derive(Default)]
pub struct Entities {
    /// The allocator responsible for creating new entity identifiers.
    id_allocator: EntityIdAllocator<EntityLocation>,
    /// Maps the components of an archetype to the ID of that archetype.
    archetype_ids: Map<Box<ArchetypeComponents>, ArchetypeId>,
    /// The archetypes that are responsible for storing entities.
    ///
    /// This is indexed by [`ArchetypeId`]s.
    archetypes: Vec<ArchetypeStorage>,
}

impl Entities {
    // ========================================================================================== //
    // INTERNAL ACCESSES                                                                          //
    // ========================================================================================== //

    /// Returns a shared reference to the [`EntityId`] allocator used to
    /// create new [`EntityId`]s for this [`Entities`].
    ///
    /// [`EntityId`]: crate::entities::EntityId
    #[inline(always)]
    pub fn id_allocator(&self) -> &EntityIdAllocator<EntityLocation> {
        &self.id_allocator
    }

    /// Returns a mutable reference to the [`EntityId`] allocator used to
    /// create new [`EntityId`]s for this [`Entities`].
    ///
    /// # Safety
    ///
    /// The caller must not modify the allocator in a way that would invalidate existing entity
    /// IDs.
    ///
    /// [`EntityId`]: crate::entities::EntityId
    #[inline(always)]
    pub unsafe fn id_allocator_mut(&mut self) -> &mut EntityIdAllocator<EntityLocation> {
        &mut self.id_allocator
    }

    /// Returns the storage that is responsible for storing entities with a given [`ArchetypeId`].
    #[inline(always)]
    pub fn archetype_storages(&self) -> &[ArchetypeStorage] {
        &self.archetypes
    }

    /// Returns the storage that is responsible for storing entities with a given [`ArchetypeId`].
    ///
    /// # Safety
    ///
    /// The caller must not modify a storage in a way that would invalidate existing entity
    /// locations.
    #[inline(always)]
    pub unsafe fn archetype_storage(&mut self) -> &mut [ArchetypeStorage] {
        &mut self.archetypes
    }

    // ========================================================================================== //
    // ENTITY MANAGEMENT                                                                          //
    // ========================================================================================== //

    /// Returns the archetype ID for the given components.
    pub fn get_archetype_id<C>(&mut self, components: C) -> ArchetypeId
    where
        C: AsRef<ArchetypeComponents> + Into<Box<ArchetypeComponents>>,
    {
        match self
            .archetype_ids
            .raw_entry_mut()
            .from_key(components.as_ref())
        {
            hashbrown::hash_map::RawEntryMut::Occupied(e) => {
                // An archetype for this set of components already exists. We can just insert
                // the entity in it.
                *e.get()
            }
            hashbrown::hash_map::RawEntryMut::Vacant(e) => {
                // No archetype exists yet for this set of components. We need to create a new
                // archetype and insert it into the collection.
                let id: ArchetypeId = self.archetypes.len();
                e.insert(components.into(), id);
                id
            }
        }
    }

    /// Spawns a new entity in the collection.
    ///
    /// # Returns
    ///
    /// This function returns an exclusive reference to the inserted entity.
    pub fn spawn<C>(&mut self, components: C) -> EntityMut
    where
        C: ComponentList,
    {
        let archetype = self.get_archetype_id(components.archetype_components());

        // SAFETY: `get_archetype_id` always return valid archetype IDs.
        let storage = unsafe { self.archetypes.get_unchecked_mut(archetype) };
        storage.reserve_one();

        let index = storage.len();
        let location = EntityLocation { archetype, index };
        let id = self.id_allocator.allocate(location);

        // SAFETY: We called `reserve_one` previously, and the storage we selected is the right
        // one.
        unsafe { storage.push_assume_capacity(id.index(), components) };

        // SAFETY: We just inserted the entity into the collection.
        unsafe { EntityMut::from_raw_parts(self, id.index()) }
    }

    /// Despawns the entity at the provided index.
    ///
    /// # Safety
    ///
    /// The provided index must reference a currently live entity.
    pub unsafe fn despawn_unchecked(&mut self, index: EntityIndex) {
        // SAFETY: The caller must ensure that the entity is live.
        let location = unsafe { *self.id_allocator.deallocate_unchecked(index) };

        // SAFETY: Stored locations are always valid.
        let storage = unsafe { self.archetypes.get_unchecked_mut(location.archetype) };

        // Remove the entity from its archetype storage.

        // SAFETY: Stored locations are always valid.
        let removed_entity_index = unsafe { storage.swap_remove_unchecked(location.index) };
        debug_assert_eq!(removed_entity_index, index);

        // An entity has been moved in the place of the removed entity. We need to update
        // its location. That only happens when the removed entity is not the last one.

        if location.index == storage.len() {
            return;
        }

        // SAFETY: We made sure to handle the case where the entity is the last one, meaning that
        // the index is still valid.
        let moved_entity_index = unsafe { *storage.entity_indices().get_unchecked(location.index) };

        // SAFETY: The moved entity is live.
        unsafe { *self.id_allocator.get_unchecked_mut(moved_entity_index) = location };
    }

    // ========================================================================================== //
    // ENTITY ACCESS
    // ========================================================================================== //

    /// Gets a reference to one of the entities in the collection.
    ///
    /// # Safety
    ///
    /// The caller is responsible for providing a valid [`EntityIndex`] that references
    /// a live entity.
    #[inline(always)]
    pub unsafe fn get_entity_unchecked_mut(&mut self, index: EntityIndex) -> EntityMut {
        unsafe { EntityMut::from_raw_parts(self, index) }
    }

    /// Gets a reference to one of the entities in the collection.
    ///
    /// # Safety
    ///
    /// The caller is responsible for providing a valid [`EntityIndex`] that references
    /// a live entity.
    #[inline(always)]
    pub unsafe fn get_entity_unchecked(&self, index: EntityIndex) -> EntityRef {
        unsafe { EntityRef::from_raw_parts(self, index) }
    }

    /// Gets an exclusive reference to an entity living in this collection.
    ///
    /// # Returns
    ///
    /// If the provided [`EntityId`] refers to a live entity, it is returned as an [`EntityMut`].
    ///
    /// Otherwise, this function returns `None`.
    pub fn get_entity_mut(&mut self, entity: EntityId) -> Option<EntityMut> {
        if self.id_allocator.is_valid(entity) {
            // SAFETY: The entity is live.
            Some(unsafe { self.get_entity_unchecked_mut(entity.index()) })
        } else {
            None
        }
    }

    /// Gets an exclusive reference to an entity living in this collection.
    ///
    /// # Panics
    ///
    /// This function panics if the provided [`EntityId`] does not refer to a live entity.
    #[track_caller]
    pub fn entity_mut(&mut self, entity: EntityId) -> EntityMut {
        if self.id_allocator.is_valid(entity) {
            // SAFETY: The entity is live.
            unsafe { self.get_entity_unchecked_mut(entity.index()) }
        } else {
            invalid_entity_id(entity)
        }
    }

    /// Gets a shared reference to an entity living in this collection.
    ///
    /// # Returns
    ///
    /// If the provided [`EntityId`] refers to a live entity, it is returned as an [`EntityRef`].
    ///
    /// Otherwise, this function returns `None`.
    pub fn get_entity(&self, entity: EntityId) -> Option<EntityRef> {
        if self.id_allocator.is_valid(entity) {
            // SAFETY: The entity is live.
            Some(unsafe { self.get_entity_unchecked(entity.index()) })
        } else {
            None
        }
    }

    /// Gets a shared reference to an entity living in this collection.
    ///
    /// # Panics
    ///
    /// This function panics if the provided [`EntityId`] does not refer to a live entity.
    #[track_caller]
    pub fn entity(&self, entity: EntityId) -> EntityRef {
        if self.id_allocator.is_valid(entity) {
            // SAFETY: The entity is live.
            unsafe { self.get_entity_unchecked(entity.index()) }
        } else {
            invalid_entity_id(entity)
        }
    }
}

#[track_caller]
#[inline(never)]
#[cold]
fn invalid_entity_id(entity: EntityId) -> ! {
    panic!("Invalid entity ID: {:?}", entity);
}
