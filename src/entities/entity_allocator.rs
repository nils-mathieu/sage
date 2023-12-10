use core::hash::{Hash, Hasher};
use core::mem::MaybeUninit;

use alloc::vec::Vec;

/// An entity that has been instanciated in an [`Entities`] collection.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(target_pointer_width = "64", repr(align(8)))]
#[repr(C)]
pub struct Entity {
    /// The index of the entity within the [`Entities`] collection.
    index: u32,
    /// The generation of the entity.
    ///
    /// This generation is used to ensure that the entity has not been removed or replaced by
    /// another entity with the same index.
    ///
    /// Specifically, every time the entity is removed, the generation for that index is increased
    /// by one, ensuring that no two entities with a given index have the same generation.
    ///
    /// When the system runs out of generations, a panic will occur.
    generation: u32,
}

impl Entity {
    /// Turns this [`Entity`] as a `u64` instance.
    ///
    /// This function is only available when the target pointer width is 64 bits.
    #[cfg(target_pointer_width = "64")]
    #[inline(always)]
    fn cast_to_u64(p: &Self) -> &u64 {
        // SAFETY:
        //  When the target pointer width is 64 bits, we know that the `Entity` instance is
        //  aligned to 8 bytes. In that case, this cast is safe.
        unsafe { &*(p as *const Self as *const u64) }
    }

    /// Returns the index associated with this [`Entity`] instance.
    #[inline(always)]
    pub fn index(&self) -> u32 {
        self.index
    }
}

impl PartialEq for Entity {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(target_pointer_width = "64")]
        {
            Self::cast_to_u64(self) == Self::cast_to_u64(other)
        }

        #[cfg(not(target_pointer_width = "64"))]
        {
            self.index == other.index && self.generation == other.generation
        }
    }
}

impl Eq for Entity {}

impl Hash for Entity {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[cfg(target_pointer_width = "64")]
        {
            Self::cast_to_u64(self).hash(state)
        }

        #[cfg(not(target_pointer_width = "64"))]
        {
            self.index.hash(state);
            self.generation.hash(state);
        }
    }
}

/// Stores metadata information about an [`Entity`] allocated by an [`EntityAllocator`].
struct EntityMeta<T> {
    /// A boolean indicating whether the entity is live or not.
    live: bool,
    /// The payload of the [`Entity`] instance.
    payload: MaybeUninit<T>,
    /// The current generation of the [`Entity`] instance, if it is live.
    ///
    /// If `payload` is [`None`], meaning that the entity is actually dead, this field is
    /// the generation of the *next* [`Entity`] instance that will be allocated at the same index.
    generation: u32,
}

impl<T> EntityMeta<T> {
    /// Creates a new [`EntityMeta`] instance with the provided payload.
    pub fn new(payload: T) -> Self {
        Self {
            live: true,
            payload: MaybeUninit::new(payload),
            generation: 0,
        }
    }

    /// Returns the payload of this [`EntityMeta`] instance.
    ///
    /// # Safety
    ///
    /// This function assumes that the [`EntityMeta`] instance is live, meaning that its
    /// `payload` field is initialized.
    #[inline(always)]
    pub unsafe fn payload_unchecked(&self) -> &T {
        self.payload.assume_init_ref()
    }

    /// Returns the payload of this [`EntityMeta`] instance.
    ///
    /// # Safety
    ///
    /// This function assumes that the [`EntityMeta`] instance is live, meaning that its
    /// `payload` field is initialized.
    #[inline(always)]
    pub unsafe fn payload_unchecked_mut(&mut self) -> &mut T {
        self.payload.assume_init_mut()
    }

    /// Marks this [`EntityMeta`] instance as live, inserting the provided payload into
    /// it.
    ///
    /// # Safety
    ///
    /// This function assumes that the [`EntityMeta`] instance is dead, meaning that its
    /// `payload` field is not initialized.
    ///
    /// Calling this function while this invariant is not respected does not result in
    /// undefined behavior, but the eventual pervious payload will be leaked.
    #[inline]
    pub fn allocate_unchecked(&mut self, payload: T) {
        self.payload.write(payload);
    }

    /// Marks this [`EntityMeta`] instance as dead, returning the payload that was previously
    /// stored in it.
    ///
    /// # Safety
    ///
    /// This function assumes that the [`EntityMeta`] instance is live, meaning that its
    /// `payload` field is initialized.
    #[inline]
    pub unsafe fn deallocate_unchecked(&mut self) -> T {
        self.live = false;

        // We're deallocating the entity, so we need to increase the generation
        // number to ensure that no two entities with the same index have the same
        // generation.
        self.generation = self
            .generation
            .checked_add(1)
            .expect("the generation number overflowed");

        self.payload.assume_init_read()
    }
}

/// An allocator for [`Entity`] instances.
pub struct EntityAllocator<T> {
    /// The generation numbers of the [`Entity`] instances that have been allocated by this
    /// instance.
    entities: Vec<EntityMeta<T>>,
    /// The list of indices that are currently free.
    ///
    /// This information is redundant with the `live` field of the [`EntityMeta`] instances,
    /// but using a separate list allows the allocation to be done in constant time rather
    /// than linear time.
    ///
    /// If an index is present in this list, it means that the corresponding [`Entity`] instance
    /// is guaranteed to be dead, meaning that the `payload` field of the corresponding
    /// [`EntityMeta`] instance is not initialized.
    free_indices: Vec<u32>,
}

impl<T> EntityAllocator<T> {
    /// Creates a new [`EntityAllocator`] instance.
    #[inline]
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
            free_indices: Vec::new(),
        }
    }

    /// Allocates a new [`Entity`] instance, associating the provided payload with it.
    pub fn allocate(&mut self, payload: T) -> Entity {
        if let Some(free_index) = self.free_indices.pop() {
            // SAFETY: we know that the `free_indices` list contains valid indices into the
            //  `entities` array.
            let meta = unsafe { self.entities.get_unchecked_mut(free_index as usize) };

            // SAFETY: we know that the `meta` instance is dead (because it comes from the free
            // list).
            meta.allocate_unchecked(payload);

            Entity {
                index: free_index,
                generation: meta.generation,
            }
        } else {
            // The free list was empty.
            // We need to allocate a new entity entry.
            let index = self.entities.len() as u32;

            self.entities.push(EntityMeta::new(payload));

            Entity {
                index,
                generation: 0,
            }
        }
    }

    /// Returns whether the provided [`Entity`] is live or not.
    pub fn is_live(&self, entity: Entity) -> bool {
        match self.entities.get(entity.index as usize) {
            Some(meta) => meta.live && meta.generation == entity.generation,
            None => false,
        }
    }

    /// Deallocates the provided [`Entity`] instance, returning the payload that was associated
    /// with it.
    ///
    /// # Safety
    ///
    /// This function assumes that the provided entity index is valid and live.
    #[inline]
    pub unsafe fn deallocate_unchecked(&mut self, index: u32) -> T {
        self.free_indices.push(index);
        self.entities
            .get_unchecked_mut(index as usize)
            .deallocate_unchecked()
    }

    /// Returns the metadata associated with the provided [`Entity`] instance.
    ///
    /// # Safety
    ///
    /// This entity must have been allocated by this [`EntityAllocator`] instance, and it must
    /// still be live.
    #[inline]
    pub fn get_unchecked(&self, index: u32) -> &T {
        // SAFETY: we know that the provided index is valid.
        unsafe {
            self.entities
                .get_unchecked(index as usize)
                .payload_unchecked()
        }
    }

    /// Returns the metadata associated with the provided [`Entity`] instance.
    ///
    /// # Safety
    ///
    /// This entity must have been allocated by this [`EntityAllocator`] instance, and it must
    /// still be live.
    #[inline]
    pub fn get_unchecked_mut(&mut self, index: u32) -> &mut T {
        // SAFETY: we know that the provided index is valid.
        unsafe {
            self.entities
                .get_unchecked_mut(index as usize)
                .payload_unchecked_mut()
        }
    }
}
