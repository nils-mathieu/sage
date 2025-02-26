use crate::{
    OpaquePtr, Uuid,
    entities::{Component, Entities, EntityId, EntityIndex, EntityLocation},
};

/// A reference to an entity in an [`Entities`] collection.
pub struct EntityMut<'a> {
    /// The [`Entities`] collection that the entity is stored in.
    entities: &'a mut Entities,
    /// The index of the entity. Used to access the entity's state.
    index: EntityIndex,
}

impl<'a> EntityMut<'a> {
    /// Creates a reference to an entity in an [`Entities`] collection.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the entity at `index` is live.
    #[inline]
    pub(crate) unsafe fn from_raw_parts(entities: &'a mut Entities, index: EntityIndex) -> Self {
        Self { entities, index }
    }

    // ========================================================================================== //
    // ENTITY METADATA                                                                            //
    // ========================================================================================== //

    /// Returns the location of the entity within its collection.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        unsafe { *self.entities.id_allocator().get_unchecked(self.index) }
    }

    /// Returns the raw [`EntityIndex`] of the entity.
    #[inline]
    pub fn index(&self) -> EntityIndex {
        self.index
    }

    /// Returns the ID of the entity.
    #[inline]
    pub fn id(&self) -> EntityId {
        unsafe {
            self.entities
                .id_allocator()
                .get_id_for_index_unchecked(self.index)
        }
    }

    // ========================================================================================== //
    // COMPONENT ACCESS                                                                           //
    // ========================================================================================== //

    /// Returns whether the entity has a component with the specified UUID.
    pub fn has_component_raw(&self, uuid: Uuid) -> bool {
        let location = self.location();
        unsafe {
            self.entities
                .archetype_storages()
                .get_unchecked(location.archetype)
                .has_component(uuid)
        }
    }

    /// Returns whether the entity has a component of the specified type.
    #[inline]
    pub fn has_component<C: Component>(&self) -> bool {
        self.has_component_raw(C::UUID)
    }

    /// Gets a raw pointer to one of the entity's components based on its UUID.
    ///
    /// # Returns
    ///
    /// On success, this function returns a valid pointer to the component.
    ///
    /// On failure, when the component is not part of the entity's archetype, this function returns
    /// `None`.
    pub fn get_raw(&self, uuid: Uuid) -> Option<OpaquePtr> {
        let location = self.location();
        unsafe {
            self.entities
                .archetype_storages()
                .get_unchecked(location.archetype)
                .get_column(uuid)
                .map(|column| column.get_unchecked(location.index))
        }
    }

    /// Gets a shared reference to one of the entity's components based on its UUID.
    ///
    /// If the component is not part of the entity's archetype, this function returns `None`.
    pub fn try_get<C: Component>(&self) -> Option<&C> {
        unsafe { self.get_raw(C::UUID).map(|x| x.as_ref()) }
    }

    /// Gets a mutable reference to one of the entity's components based on its UUID.
    ///
    /// If the component is not part of the entity's archetype, this function returns `None`.
    pub fn try_get_mut<C: Component>(&mut self) -> Option<&mut C> {
        unsafe { self.get_raw(C::UUID).map(|x| x.as_mut()) }
    }

    /// Gets a shared reference to one of the entity's components based on its UUID.
    ///
    /// # Panics
    ///
    /// This function panics if the component is not part of the entity's archetype.
    #[track_caller]
    pub fn get<C: Component>(&self) -> &C {
        self.try_get::<C>()
            .unwrap_or_else(|| missing_component(C::DEBUG_NAME))
    }

    /// Gets a mutable reference to one of the entity's components based on its UUID.
    ///
    /// # Panics
    ///
    /// This function panics if the component is not part of the entity's archetype.
    #[track_caller]
    pub fn get_mut<C: Component>(&mut self) -> &mut C {
        self.try_get_mut::<C>()
            .unwrap_or_else(|| missing_component(C::DEBUG_NAME))
    }

    /// Despawns the entity, removing it from the collection.
    #[inline]
    pub fn despawn(self) {
        unsafe { self.entities.despawn_unchecked(self.index) };
    }
}

/// A shared reference to an existing entity in an [`Entities`] collection.
pub struct EntityRef<'a> {
    /// The [`Entities`] collection that the entity is stored in.
    entities: &'a Entities,
    /// The index of the entity. Used to access the entity's state.
    index: EntityIndex,
}

impl<'a> EntityRef<'a> {
    /// Creates a new [`EntityRef`] instance from the provided [`Entities`] collection and entity
    /// index.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the entity at `index` is live.
    #[inline(always)]
    pub(crate) unsafe fn from_raw_parts(entities: &'a Entities, index: EntityIndex) -> Self {
        Self { entities, index }
    }

    // ========================================================================================== //
    // ENTITY METADATA                                                                            //
    // ========================================================================================== //

    /// Returns the location of the entity within its collection.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        unsafe { *self.entities.id_allocator().get_unchecked(self.index) }
    }

    /// Returns the raw [`EntityIndex`] of the entity.
    #[inline]
    pub fn index(&self) -> EntityIndex {
        self.index
    }

    /// Returns the ID of the entity.
    #[inline]
    pub fn id(&self) -> EntityId {
        unsafe {
            self.entities
                .id_allocator()
                .get_id_for_index_unchecked(self.index)
        }
    }

    // ========================================================================================== //
    // COMPONENT ACCESS                                                                           //
    // ========================================================================================== //

    /// Returns whether the entity has a component with the specified UUID.
    pub fn has_component_raw(&self, uuid: Uuid) -> bool {
        let location = self.location();
        unsafe {
            self.entities
                .archetype_storages()
                .get_unchecked(location.archetype)
                .has_component(uuid)
        }
    }

    /// Returns whether the entity has a component of the specified type.
    #[inline]
    pub fn has_component<C: Component>(&self) -> bool {
        self.has_component_raw(C::UUID)
    }

    /// Gets a raw pointer to one of the entity's components based on its UUID.
    ///
    /// # Returns
    ///
    /// On success, this function returns a valid pointer to the component.
    ///
    /// On failure, when the component is not part of the entity's archetype, this function returns
    /// `None`.
    pub fn get_raw(&self, uuid: Uuid) -> Option<OpaquePtr> {
        let location = self.location();
        unsafe {
            self.entities
                .archetype_storages()
                .get_unchecked(location.archetype)
                .get_column(uuid)
                .map(|column| column.get_unchecked(location.index))
        }
    }

    /// Gets a shared reference to one of the entity's components based on its UUID.
    ///
    /// If the component is not part of the entity's archetype, this function returns `None`.
    pub fn try_get<C: Component>(&self) -> Option<&C> {
        unsafe { self.get_raw(C::UUID).map(|x| x.as_ref()) }
    }

    /// Gets a shared reference to one of the entity's components based on its UUID.
    ///
    /// # Panics
    ///
    /// This function panics if the component is not part of the entity's archetype.
    #[track_caller]
    pub fn get<C: Component>(&self) -> &C {
        self.try_get::<C>()
            .unwrap_or_else(|| missing_component(C::DEBUG_NAME))
    }
}

#[cold]
#[track_caller]
#[inline(never)]
fn missing_component(name: &'static str) -> ! {
    panic!("Entity does not have the requested component: {name:?}")
}
