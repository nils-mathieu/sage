use {
    super::{ComponentRegistry, EntityRow, modify_entity::ModifyEntity},
    crate::entities::{
        ArchetypeComponents, ArchetypeStorage, ComponentList, EntityId, EntityIdAllocator,
        EntityIndex, EntityMut, EntityRef,
    },
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
    pub row: EntityRow,
}

/// The map type used by the [`Entities`] collection.
type Map<K, V> = hashbrown::HashMap<K, V, foldhash::fast::FixedState>;

/// A collection of entities that are stored efficiently in memory and grouped using the
/// components that make them up.
#[derive(Default)]
pub struct Entities {
    /// The components that are stored in the collection.
    components: ComponentRegistry,
    /// The allocator responsible for creating new entity identifiers.
    id_allocator: EntityIdAllocator,
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
    pub fn id_allocator(&self) -> &EntityIdAllocator {
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
    pub unsafe fn id_allocator_mut(&mut self) -> &mut EntityIdAllocator {
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

    /// Flushes the reserved entities, giving theme an actual location in the collection.
    pub fn flush(&mut self) {
        unsafe {
            let archetype = self.get_archetype_id(ArchetypeComponents::EMPTY);

            // SAFETY: `get_archetype_id` always return valid archetype IDs.
            let storage = self.archetypes.get_unchecked_mut(archetype);

            // Reserve the correct number of entities.
            storage.reserve(self.id_allocator.reserved_entities());

            // SAFETY: The callback does not panic.
            self.id_allocator.flush(|id| {
                let row = storage.len();
                storage.push_assume_capacity(id.index(), ());
                EntityLocation { row, archetype }
            });
        }
    }

    /// Returns the archetype ID for the given components.
    ///
    /// # Safety
    ///
    /// The components present in the provided [`ArchetypeComponents`] must have been
    /// registered previously.
    pub unsafe fn get_archetype_id<C>(&mut self, components: C) -> ArchetypeId
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
                self.archetypes.push(ArchetypeStorage::new(
                    components
                        .as_ref()
                        .as_uuids()
                        .iter()
                        .map(|&id| unsafe { self.components.get_by_uuid(id).unwrap_unchecked() }),
                ));
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
        self.flush();

        let mut archetype_components = Vec::new();
        components.register(&mut self.components, &mut |info| {
            archetype_components.push(info.uuid);
        });

        // SAFETY: `register` only registers components once. This means that the only thing we
        // need to do is sort the vector.
        let archetype_components = unsafe {
            archetype_components.sort_unstable();
            ArchetypeComponents::from_boxed_slice_unchecked(archetype_components.into_boxed_slice())
        };

        let archetype = unsafe { self.get_archetype_id(archetype_components) };

        // SAFETY: `get_archetype_id` always return valid archetype IDs.
        let storage = unsafe { self.archetypes.get_unchecked_mut(archetype) };
        storage.reserve_one();

        let row = storage.len();
        let location = EntityLocation { archetype, row };
        let id = unsafe { self.id_allocator.allocate(location) };

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
        self.flush();

        // SAFETY: The caller must ensure that the entity is live.
        let location = unsafe { *self.id_allocator.deallocate_unchecked(index) };

        // SAFETY: Stored locations are always valid.
        let storage = unsafe { self.archetypes.get_unchecked_mut(location.archetype) };

        // Remove the entity from its archetype storage.

        // SAFETY: Stored locations are always valid.
        let removed_entity_index = unsafe { storage.swap_remove_unchecked(location.row) };
        debug_assert_eq!(removed_entity_index, index);

        // An entity has been moved in the place of the removed entity. We need to update
        // its location. That only happens when the removed entity is not the last one.

        if location.row == storage.len() {
            return;
        }

        // SAFETY: We made sure to handle the case where the entity is the last one, meaning that
        // the index is still valid.
        let moved_entity_index = unsafe { *storage.entity_indices().get_unchecked(location.row) };

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

    // ========================================================================================== //
    // ENTITY MODIFICATIONS                                                                       //
    // ========================================================================================== //

    /// Modifies an entity's components.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided `entity` is live.
    pub unsafe fn modify_unchecked<M>(&mut self, entity: EntityIndex, modify: M) -> M::Output
    where
        M: ModifyEntity,
    {
        // SAFETY: The caller must provide a valid entity.
        let old_location = unsafe { *self.id_allocator.get_unchecked(entity) };

        // SAFETY: Stored locations are always valid.
        let old_storage = unsafe { self.archetypes.get_unchecked(old_location.archetype) };

        let old_archetype = old_storage.archetype_components();
        let new_archetype = modify.modify_archetype(&mut self.components, old_archetype);

        // SAFETY: `modify_archetype` will register the necessary components.
        let new_archetype_id = unsafe { self.get_archetype_id(new_archetype) };

        if new_archetype_id == old_location.archetype {
            let old_storage = unsafe { self.archetypes.get_unchecked_mut(old_location.archetype) };

            // The entity won't change archetypes. We can just modify it in place.
            unsafe { modify.modify_in_place(old_storage.get(old_location.row)) }
        } else {
            unsafe {
                // SAFETY: `get_archetype_id` returns valid archetype IDs.
                // NOTE: We need to use raw pointers to ensure we're splitting the borrows
                // correctly.
                let old_storage = &mut *self.archetypes.as_mut_ptr().add(old_location.archetype);
                let new_storage = &mut *self.archetypes.as_mut_ptr().add(new_archetype_id);

                new_storage.reserve_one();
                let new_row = new_storage.len();

                let out =
                    modify.modify(old_storage.get(old_location.row), new_storage.get(new_row));

                // We need to:
                // 1. Swap-remove the source entity (now that it has been moved out).
                // 2. Update the location of the entity that was moved (if the entity was not the
                //    last one).
                // 3. Increase the length of the destination storage by one.
                // 4. Update the entity's location.
                let removed = new_storage.swap_remove_unchecked_no_drop(old_location.row);
                debug_assert_eq!(removed, entity);

                if old_location.row != old_storage.len() {
                    let moved_entity =
                        *new_storage.entity_indices().get_unchecked(old_location.row);
                    self.id_allocator.get_unchecked_mut(moved_entity).row = old_location.row;
                }

                new_storage.assume_pushed(entity);

                *self.id_allocator.get_unchecked_mut(entity) = EntityLocation {
                    archetype: new_archetype_id,
                    row: new_row,
                };

                out
            }
        }
    }
}

#[track_caller]
#[inline(never)]
#[cold]
fn invalid_entity_id(entity: EntityId) -> ! {
    panic!("Invalid entity ID: {:?}", entity);
}
