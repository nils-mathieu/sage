use std::num::NonZeroU32;

/// The raw entity index part of an [`EntityId`].
pub type EntityIndex = u32;

/// A cheaply-clonable identifier for an entity.
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub struct EntityId {
    /// The index of the entity.
    ///
    /// This is the index of the entity's [`Slot`] responsible for storing metadata about the
    /// entity.
    #[cfg(target_endian = "little")]
    index: EntityIndex,
    /// The generation number of the entity's ID.
    generation: NonZeroU32,
    #[cfg(target_endian = "big")]
    index: EntityIndex,
}

impl EntityId {
    /// An [`EntityId`] that is unlikely to be valid.
    ///
    /// Can be used as a placeholder.
    pub const DUMMY: Self = Self {
        index: u32::MAX,
        generation: NonZeroU32::MAX,
    };

    /// Turns this [`EntityId`] instance into its bit representation.
    ///
    /// This should generally not be used to inspect the entity's internals.
    #[inline(always)]
    pub fn to_bits(self) -> u64 {
        unsafe { std::mem::transmute(self) }
    }

    /// Returns the raw [`EntityIndex`] associated with this [`EntityId`] instance.
    ///
    /// This is useful when an entity is known to be valid and must be kept around. An
    /// [`EntityIndex`] cannot be checked correctly in this [`Entities`] collection but is only
    /// four bytes instead of eight.
    #[inline(always)]
    pub fn index(self) -> EntityIndex {
        self.index
    }
}

impl std::cmp::PartialEq for EntityId {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl std::cmp::Eq for EntityId {}

impl std::cmp::PartialOrd for EntityId {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for EntityId {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_bits().cmp(&other.to_bits())
    }
}

impl std::fmt::Debug for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}v{}", self.index, self.generation.get())
    }
}

impl std::hash::Hash for EntityId {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state)
    }
}

/// Stores metadata about an entity that has been allocated in an [`Entities`] collection.
struct Slot<M> {
    /// The current generation of this slot.
    ///
    /// This is checked against an [`Entity`]'s generation number to ensure that the entity
    /// is still valid and has not been deleted.
    generation: NonZeroU32,
    /// The metadata stored in this slot.
    metadata: M,
}

impl<M> Slot<M> {
    /// Bumps the generation number of this slot.
    pub fn bump_generation(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .unwrap_or_else(|| too_many_entities());
    }
}

/// A potentially concurrent collection responsible for creating new [`EntityId`]s.
pub struct EntityIdAllocator<M> {
    /// The slots that store metadata about entities.
    slots: Vec<Slot<M>>,
    /// The free list of slots that have been deallocated so far.
    ///
    /// When new entities are created, they are allocated from this list in priority.
    free_list: Vec<u32>,
}

impl<M> EntityIdAllocator<M> {
    /// Allocates a new entity with the provided metadata.
    pub fn allocate(&mut self, metadata: M) -> EntityId {
        if let Some(index) = self.free_list.pop() {
            let slot = unsafe { self.slots.get_unchecked(index as usize) };
            let generation = slot.generation;
            EntityId { index, generation }
        } else {
            let index = self
                .slots
                .len()
                .try_into()
                .unwrap_or_else(|_| too_many_entities());
            let generation = NonZeroU32::MIN;
            self.slots.push(Slot {
                generation,
                metadata,
            });
            EntityId { index, generation }
        }
    }

    /// Returns whether the entity with the provided identifier is valid or not.
    pub fn is_valid(&self, entity: EntityId) -> bool {
        self.slots
            .get(entity.index as usize)
            .is_some_and(|slot| slot.generation == entity.generation)
    }

    /// Deallocates the entity with the provided identifier without checking whether the entity
    /// is actually valid or not.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the entity at `entity` is valid and live.
    pub unsafe fn deallocate_unchecked(&mut self, entity: EntityIndex) -> &mut M {
        let slot = unsafe { self.slots.get_unchecked_mut(entity as usize) };
        slot.bump_generation();
        self.free_list.push(entity);
        &mut slot.metadata
    }

    /// Attempts to deallocate the entity with the provided identifier.
    ///
    /// # Returns
    ///
    /// If the entity exists, then this function returns the metadata associated with the entity.
    ///
    /// Otherwise, this function returns `None`.
    pub fn deallocate(&mut self, entity: EntityId) -> Option<&mut M> {
        let slot = self
            .slots
            .get_mut(entity.index as usize)
            .filter(|slot| slot.generation == entity.generation)?;
        slot.bump_generation();
        self.free_list.push(entity.index);
        Some(&mut slot.metadata)
    }

    /// Returns the [`EntityId`] associated with the provided index without checking whether
    /// it is valid or not.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided index is valid and live.
    #[inline]
    pub unsafe fn get_id_for_index_unchecked(&self, index: EntityIndex) -> EntityId {
        let generation = unsafe { self.slots.get_unchecked(index as usize).generation };
        EntityId { index, generation }
    }

    /// Returns the metadata associated with the provided entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the entity is valid and live.
    #[inline]
    pub unsafe fn get_unchecked(&self, entity: EntityIndex) -> &M {
        unsafe { &self.slots.get_unchecked(entity as usize).metadata }
    }

    /// Returns the mutable metadata associated with the provided entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the entity is valid and live.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, entity: EntityIndex) -> &mut M {
        unsafe { &mut self.slots.get_unchecked_mut(entity as usize).metadata }
    }

    /// Returns the metadata associated with the provided entity.
    ///
    /// # Returns
    ///
    /// If the entity exists, then this function returns the metadata associated with the entity.
    ///
    /// Otherwise, this function returns `None`.
    pub fn get(&self, entity: EntityId) -> Option<&M> {
        self.slots
            .get(entity.index as usize)
            .filter(|slot| slot.generation == entity.generation)
            .map(|slot| &slot.metadata)
    }

    /// Returns the mutable metadata associated with the provided entity.
    ///
    /// # Returns
    ///
    /// If the entity exists, then this function returns the mutable metadata associated with the
    /// entity.
    ///
    /// Otherwise, this function returns `None`.
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut M> {
        self.slots
            .get_mut(entity.index as usize)
            .filter(|slot| slot.generation == entity.generation)
            .map(|slot| &mut slot.metadata)
    }
}

impl<M> Default for EntityIdAllocator<M> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
        }
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn too_many_entities() -> ! {
    panic!("Too many entities have been allocated/deallocated");
}
