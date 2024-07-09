//! The logic behind the [`Entity`] identifier.

use alloc::vec::Vec;
use core::{
    num::NonZero,
    sync::atomic::{AtomicIsize, Ordering::Relaxed},
};

mod id;
pub use self::id::*;

/// Stores information about an [`Entity`] identifier.
struct Slot<T> {
    /// The metadata associated with the entity.
    ///
    /// This is only initialized when the `next_free` field is not `OCCUPIED`.
    metadata: T,
    /// The generation number of the slot.
    generation: NonZero<u32>,
}

/// A slot in the entity allocator.
pub struct EntityAllocator<T> {
    /// The slots that have been properly allocated so far.
    ///
    /// The `index` part of the `Entity` type is an index within this vector.
    slots: Vec<Slot<T>>,
    /// The list of free slots.
    free_list: Vec<u32>,

    /// The cursor that indicates how many slots have been *reserved* so far.
    reserve_cursor: AtomicIsize,
}

impl<T> EntityAllocator<T> {
    /// Creates a new [`EntityAllocator`] instance with no entities.
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
            reserve_cursor: AtomicIsize::new(0),
        }
    }

    /// Decrements the `reserve_cursor` by `count` and returns the new value.
    ///
    /// # Panics
    ///
    /// This function panics if it would underflow `isize::MIN`.
    fn reserve_raw(&self, count: usize) -> isize {
        let mut current = self.reserve_cursor.load(Relaxed);

        loop {
            let new = current
                .checked_sub_unsigned(count)
                .unwrap_or_else(|| too_many_entities());

            match self
                .reserve_cursor
                .compare_exchange_weak(current, new, Relaxed, Relaxed)
            {
                Ok(_) => return new,
                Err(next) => current = next,
            }
        }
    }

    /// Reserves one entity in advance without needing exclusive access to the `EntityAllocator<T>`
    /// instance.
    ///
    /// This method is lock-free and can be called concurrently.
    pub fn reserve_one(&self) -> Entity {
        let cursor = self.reserve_raw(1);

        if cursor >= 0 {
            let index = unsafe { *self.free_list.get_unchecked(cursor as usize) };
            let slot = unsafe { self.slots.get_unchecked(index as usize) };
            Entity::new(index, slot.generation)
        } else {
            let index = self
                .slots
                .len()
                .checked_add(unsafe { cursor.unsigned_abs().unchecked_sub(1) })
                .unwrap_or_else(|| too_many_entities());
            Entity::new(index as u32, NonZero::<u32>::MIN)
        }
    }

    /// Reserves multiple entities in advance without needing exclusive access to the
    /// `EntityAllocator<T>` instance.
    ///
    /// This method is lock-free and can be called concurrently.
    ///
    /// This is like calling `reserve_one` multiple times, but more efficient. Note that entities
    /// are reserved regardless of whether the iterator is consumed or not.
    ///
    /// # Returns
    ///
    /// An iterator over the entities that were reserved.
    pub fn reserve_multiple(&self, count: usize) -> ReserveMultiple<T> {
        let cursor = self.reserve_raw(count);
        let prev_cursor = unsafe { cursor.unchecked_add(count as isize) };

        let reused_start = cursor.max(0) as usize;
        let reused_end = prev_cursor.max(0) as usize;
        let reused_slots = unsafe { self.free_list.get_unchecked(reused_start..reused_end) };

        let new_end = cursor
            .min(0)
            .unsigned_abs()
            .checked_add(self.slots.len())
            .and_then(|x| x.try_into().ok())
            .unwrap_or_else(|| too_many_entities());
        let new_start = prev_cursor
            .min(0)
            .unsigned_abs()
            .checked_add(self.slots.len())
            .and_then(|x| x.try_into().ok())
            .unwrap_or_else(|| too_many_entities());

        ReserveMultiple {
            slots: &self.slots,
            reused_slots: reused_slots.iter(),
            new_slots: new_start..new_end,
        }
    }

    /// Returns the number of entities that are pending for allocation (reserved but not yet
    /// flushed).
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn reserved(&mut self) -> usize {
        self.free_list
            .len()
            .wrapping_sub(*self.reserve_cursor.get_mut() as usize)
    }

    /// Returns whether the entity allocator needs to be flushed.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn needs_flush(&mut self) -> bool {
        *self.reserve_cursor.get_mut() != self.free_list.len() as isize
    }

    /// Flushes the entity allocator, providing the metadata of each entity that was previously
    /// reserved.
    ///
    /// You can determine how many entities will be flushed by calling `reserved` before calling
    /// this method.
    pub fn flush(&mut self, mut allocate: impl FnMut(Entity) -> T) {
        let cursor = *self.reserve_cursor.get_mut();

        let new_slots_count = cursor.min(0).unsigned_abs();
        let reused_start = cursor.max(0) as usize;

        self.slots.reserve(new_slots_count);

        for &index in unsafe { self.free_list.get_unchecked(reused_start..).iter().rev() } {
            let slot = unsafe { self.slots.get_unchecked_mut(index as usize) };
            slot.metadata = allocate(Entity::new(index, slot.generation));
        }

        for _ in 0..new_slots_count {
            let index = self
                .slots
                .len()
                .try_into()
                .unwrap_or_else(|_| too_many_entities());
            self.slots.push(Slot {
                metadata: allocate(Entity::new(index, NonZero::<u32>::MIN)),
                generation: NonZero::<u32>::MIN,
            });
        }

        self.free_list.truncate(reused_start);
        *self.reserve_cursor.get_mut() = reused_start as isize;
    }

    /// Allocates an entity.
    ///
    /// # Remarks
    ///
    /// This function must be called when the [`EntityAllocator`] does not need to be flushed. If
    /// this is not verified, then the behavior is unspecified (but safe).
    ///
    /// # Returns
    ///
    /// The allocated entity.
    pub fn allocate(&mut self, metadata: T) -> Entity {
        debug_assert!(!self.needs_flush());
        if let Some(index) = self.free_list.pop() {
            *self.reserve_cursor.get_mut() = self.free_list.len() as isize;
            let slot = unsafe { self.slots.get_unchecked_mut(index as usize) };
            slot.metadata = metadata;
            Entity::new(index, slot.generation)
        } else {
            let index = self
                .slots
                .len()
                .try_into()
                .unwrap_or_else(|_| too_many_entities());
            self.slots.push(Slot {
                metadata,
                generation: NonZero::<u32>::MIN,
            });
            Entity::new(index, NonZero::<u32>::MIN)
        }
    }

    /// Deallocates an entity.
    ///
    /// # Remarks
    ///
    /// This function must be called when the [`EntityAllocator`] does not need to be flushed. If
    /// this is not verified, then the behavior is unspecified (but safe).
    ///
    /// # Returns
    ///
    /// This function returns whether the entity was successfully deallocated. Specifically, if
    /// the provided entity is not valid, then this function returns `false`.
    pub fn deallocate(&mut self, entity: Entity) -> bool {
        debug_assert!(!self.needs_flush());
        let Some(slot) = self.slots.get_mut(entity.index() as usize) else {
            return false;
        };
        if slot.generation.get() != entity.generation() {
            return false;
        }
        self.free_list.push(entity.index());
        slot.generation = slot
            .generation
            .checked_add(1)
            .unwrap_or_else(|| too_many_entities());
        *self.reserve_cursor.get_mut() = self.free_list.len() as isize;
        true
    }

    /// Gets the metadata associated with the provided [`Entity`].
    ///
    /// Note that unflushed entities are not considered valid.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.slots
            .get(entity.index() as usize)
            .filter(|s| s.generation.get() == entity.generation())
            .map(|s| &s.metadata)
    }

    /// Gets the metadata associated with the provided [`Entity`] mutably.
    ///
    /// Note that unflushed entities are not considered valid.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.slots
            .get_mut(entity.index() as usize)
            .filter(|s| s.generation.get() == entity.generation())
            .map(|s| &mut s.metadata)
    }

    /// Returns the number of entities currently live.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn count(&self) -> usize {
        unsafe { self.slots.len().unchecked_sub(self.free_list.len()) }
    }
}

impl<T> Default for EntityAllocator<T> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}

#[inline(never)]
#[cold]
fn too_many_entities() -> ! {
    panic!("too many entities have been created")
}

/// An iterator over the entities that were reserved in advance using
/// [`EntityAllocator::reserve_multiple`].
pub struct ReserveMultiple<'a, T> {
    /// The slots in the entity allocator.
    slots: &'a [Slot<T>],
    /// The free slots that are being re-used.
    reused_slots: core::slice::Iter<'a, u32>,
    /// The new slots that have been allocated.
    new_slots: core::ops::Range<u32>,
}

impl<'a, T> Iterator for ReserveMultiple<'a, T> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.reused_slots
            .next_back()
            .map(|&i| {
                Entity::new(i, unsafe {
                    self.slots.get_unchecked(i as usize).generation
                })
            })
            .or_else(|| {
                self.new_slots
                    .next()
                    .map(|i| Entity::new(i, NonZero::<u32>::MIN))
            })
    }
}

#[cfg(test)]
mod test {
    use super::EntityAllocator;

    #[test]
    fn empty() {
        let e = EntityAllocator::<()>::new();
        assert_eq!(e.count(), 0);
    }

    #[test]
    fn reserve_one() {
        let mut e = EntityAllocator::<&str>::new();

        let a = e.reserve_one();
        let b = e.reserve_one();
        let c = e.reserve_one();

        assert_eq!(a.index(), 0);
        assert_eq!(a.generation(), 1);
        assert_eq!(b.index(), 1);
        assert_eq!(b.generation(), 1);
        assert_eq!(c.index(), 2);
        assert_eq!(c.generation(), 1);

        assert_eq!(e.reserved(), 3);
        assert_eq!(e.count(), 0);
        assert!(e.needs_flush());
        e.flush(|_| "test");
        assert_eq!(e.count(), 3);
        assert_eq!(e.reserved(), 0);
        assert!(!e.needs_flush());

        assert_eq!(e.get(a), Some(&"test"));
        assert_eq!(e.get(b), Some(&"test"));
        assert_eq!(e.get(c), Some(&"test"));
    }

    #[test]
    fn reserve_multiple() {
        let mut e = EntityAllocator::<&str>::new();

        let mut iter = e.reserve_multiple(3);
        let a = iter.next().unwrap();
        let b = iter.next().unwrap();
        let c = iter.next().unwrap();
        assert!(iter.next().is_none());

        assert_eq!(a.index(), 0);
        assert_eq!(a.generation(), 1);
        assert_eq!(b.index(), 1);
        assert_eq!(b.generation(), 1);
        assert_eq!(c.index(), 2);
        assert_eq!(c.generation(), 1);

        assert!(e.needs_flush());
        assert_eq!(e.reserved(), 3);
        assert_eq!(e.count(), 0);
        e.flush(|_| "test");
        assert!(!e.needs_flush());
        assert_eq!(e.reserved(), 0);
        assert_eq!(e.count(), 3);

        assert_eq!(e.get(a), Some(&"test"));
        assert_eq!(e.get(b), Some(&"test"));
        assert_eq!(e.get(c), Some(&"test"));
    }

    #[test]
    fn deallocate() {
        let mut e = EntityAllocator::<&str>::new();

        let a = e.reserve_one();
        let b = e.reserve_one();
        let c = e.reserve_one();

        assert_eq!(e.reserved(), 3);
        e.flush(|_| "test");
        assert_eq!(e.reserved(), 0);
        assert_eq!(e.count(), 3);

        assert_eq!(e.get(a), Some(&"test"));
        assert_eq!(e.get(b), Some(&"test"));
        assert_eq!(e.get(c), Some(&"test"));

        assert!(e.deallocate(a));
        assert_eq!(e.count(), 2);
        assert!(e.deallocate(b));
        assert_eq!(e.count(), 1);
        assert!(e.deallocate(c));
        assert_eq!(e.count(), 0);

        assert_eq!(e.get(a), None);
        assert_eq!(e.get(b), None);
        assert_eq!(e.get(c), None);

        let a = e.reserve_one();
        let b = e.reserve_one();
        let c = e.reserve_one();

        assert_eq!(a.index(), 2);
        assert_eq!(a.generation(), 2);
        assert_eq!(b.index(), 1);
        assert_eq!(b.generation(), 2);
        assert_eq!(c.index(), 0);
        assert_eq!(c.generation(), 2);

        assert_eq!(e.reserved(), 3);
        e.flush(|_| "test2");
        assert_eq!(e.reserved(), 0);

        assert_eq!(e.get(a), Some(&"test2"));
        assert_eq!(e.get(b), Some(&"test2"));
        assert_eq!(e.get(c), Some(&"test2"));
    }

    #[test]
    fn allocate() {
        let mut e = EntityAllocator::<&str>::new();

        let a = e.allocate("test1");
        let b = e.allocate("test2");
        let c = e.allocate("test3");

        assert_eq!(a.index(), 0);
        assert_eq!(a.generation(), 1);
        assert_eq!(b.index(), 1);
        assert_eq!(b.generation(), 1);
        assert_eq!(c.index(), 2);
        assert_eq!(c.generation(), 1);

        assert_eq!(e.get(a), Some(&"test1"));
        assert_eq!(e.get(b), Some(&"test2"));
        assert_eq!(e.get(c), Some(&"test3"));

        e.deallocate(a);
        e.deallocate(b);
        e.deallocate(c);

        let a = e.allocate("test4");
        let b = e.allocate("test5");
        let c = e.allocate("test6");

        assert_eq!(a.index(), 2);
        assert_eq!(a.generation(), 2);
        assert_eq!(b.index(), 1);
        assert_eq!(b.generation(), 2);
        assert_eq!(c.index(), 0);
        assert_eq!(c.generation(), 2);

        assert_eq!(e.get(a), Some(&"test4"));
        assert_eq!(e.get(b), Some(&"test5"));
        assert_eq!(e.get(c), Some(&"test6"));

        assert_eq!(e.count(), 3);
        assert_eq!(e.reserved(), 0);
    }

    #[test]
    fn reserved_isize_max() {
        let mut e = EntityAllocator::<&str>::new();

        e.reserve_raw(isize::MAX as usize + 1);
        assert_eq!(e.reserved(), isize::MAX as usize + 1);
    }
}
