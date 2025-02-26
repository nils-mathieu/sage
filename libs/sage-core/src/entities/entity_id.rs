use {
    super::EntityLocation,
    std::{
        num::NonZeroU32,
        sync::atomic::{AtomicIsize, Ordering::Relaxed},
    },
};

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
pub struct EntityIdAllocator<M = EntityLocation> {
    /// The slots that store metadata about entities.
    slots: Vec<Slot<M>>,
    /// The free list of slots that have been deallocated so far.
    ///
    /// When new entities are created, they are allocated from this list in priority.
    free_list: Vec<u32>,
    /// A cursor that's used to count the number of entities that have been reserved
    /// concurrently by the allocator.
    ///
    /// When positive, this is the number in `free_list` of indices that have not yet been
    /// reserved. This means that when `reserved == free_list.len()`, then the allocator has
    /// reserved no entities.
    ///
    /// Negative values mean that the allocator has reserved indices that are outside of the free
    /// list. Negative values mean that we use indices larger than `slot.len()` to allocate new
    /// entities.
    ///
    /// When new entity indices are reserved, the `reserved` cursor is decremented.
    reserved: AtomicIsize,
}

impl<M> EntityIdAllocator<M> {
    /// Returns the number of entities that were reserved but not yet allocated properly.
    pub fn reserved_entities(&mut self) -> usize {
        let r = *self.reserved.get_mut();
        if r < 0 {
            r.unsigned_abs() + self.free_list.len()
        } else {
            self.free_list.len() - r as usize
        }
    }

    /// Flushes the allocator's reserved entities, properly giving them the metadata they
    /// should have.
    ///
    /// # Safety
    ///
    /// `get_metadata` must not panic.
    pub unsafe fn flush(&mut self, mut get_metadata: impl FnMut(EntityId) -> M) {
        let reserved = *self.reserved.get_mut();

        // Reserve more slots if necessary to make sure we do not panic later.
        if reserved < 0 {
            self.slots.reserve(reserved.unsigned_abs());
        }

        let free_list_start = reserved.max(0) as usize;
        for index in self.free_list.drain(free_list_start..) {
            let slot = unsafe { self.slots.get_unchecked_mut(index as usize) };
            slot.metadata = get_metadata(EntityId {
                index,
                generation: slot.generation,
            });
        }

        if reserved < 0 {
            let min = self.slots.len();
            let max = unsafe { min.unchecked_add(reserved.unsigned_abs()) };

            let max = max.try_into().unwrap_or_else(|_| too_many_entities());
            let min = min as u32; // Cannot fail if max could be converted.

            for index in min..max {
                self.slots.push(Slot {
                    generation: NonZeroU32::MIN,
                    metadata: get_metadata(EntityId {
                        index,
                        generation: NonZeroU32::MIN,
                    }),
                });
            }
        }

        *self.reserved.get_mut() = self.free_list.len() as isize;
    }

    /// Returns whether the allocator needs to be flushed.
    #[inline]
    pub fn needs_flush(&mut self) -> bool {
        *self.reserved.get_mut() as usize == self.free_list.len()
    }

    /// Reserves a single entity ID.
    pub fn reserve_one(&self) -> EntityId {
        let reserved = self
            .reserved
            .fetch_sub(1, Relaxed)
            .checked_sub(1)
            .unwrap_or_else(|| too_many_entities());

        if reserved >= 0 {
            unsafe {
                let index = *self.free_list.get_unchecked(reserved as usize);
                self.get_id_for_index_unchecked(index)
            }
        } else {
            // SAFETY: reserved <= -1
            //      => reserved.unsigned_abs() >= 1
            let added = unsafe { reserved.unsigned_abs().unchecked_sub(1) };

            let index = self
                .slots
                .len()
                .checked_add(added)
                .unwrap_or_else(|| too_many_entities())
                .try_into()
                .unwrap_or_else(|_| too_many_entities());

            EntityId {
                index,
                generation: NonZeroU32::MIN,
            }
        }
    }

    /// Allocates a new entity with the provided metadata.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the allocator is flushed.
    pub unsafe fn allocate(&mut self, metadata: M) -> EntityId {
        debug_assert!(!self.needs_flush());

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
    ///
    /// The caller must ensure that the allocator is flushed.
    pub unsafe fn deallocate_unchecked(&mut self, entity: EntityIndex) -> &mut M {
        debug_assert!(!self.needs_flush());

        let slot = unsafe { self.slots.get_unchecked_mut(entity as usize) };
        slot.bump_generation();
        self.free_list.push(entity);
        &mut slot.metadata
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
            reserved: AtomicIsize::new(0),
        }
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn too_many_entities() -> ! {
    panic!("Too many entities have been allocated/deallocated");
}
