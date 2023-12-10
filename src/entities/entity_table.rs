use core::alloc::Layout;
use core::ptr::NonNull;

use super::entity_layout::{EntityLayout, InitializeEntity};
use super::entity_ptr::{EntityPtr, OwnedEntity};
use super::EntitySlice;

/// An untyped list of entities.
///
/// The entities in that list all have the same archetype.
pub struct EntityTable {
    /// The layout of the entities stored in this [`EntityTable`].
    layout: EntityLayout,
    /// The data pointer for the entities in this list.
    data: NonNull<u8>,
    /// The number of entities in this list.
    len: usize,
    /// The capacity of this list.
    ///
    /// This is the total number of entities that can be stored in the list without having to
    /// reallocate.
    cap: usize,
}

// This type allows no direct access to the underlying data. It's the responsibility of the
// user to ensure that the data is properly accessed in a way that does not violate Rust's
// safety guarantees.
unsafe impl Send for EntityTable {}
unsafe impl Sync for EntityTable {}

impl EntityTable {
    /// Creates a new [`EntityTable`] instance.
    #[inline]
    pub const fn new(layout: EntityLayout) -> Self {
        Self {
            data: layout.dangling(),
            len: 0,
            cap: 0,
            layout,
        }
    }

    /// Returns the layout of the entities stored in this [`EntityTable`].
    #[inline(always)]
    pub fn layout(&self) -> &EntityLayout {
        &self.layout
    }

    /// Returns the lentgh of this list, this is the total number of entities that are currently
    /// stored in the list.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// A function that reserves memory specifically after a call to [`push`].
    ///
    /// [`push`]: Self::push
    #[inline(never)]
    fn rallocate_for_push(&mut self) {
        if self.layout.size() == 0 {
            // This is a zero-sized component, we don't need to allocate any memory.
            return;
        }

        if self.cap == 0 {
            // This is the first time we're allocating any memory.
            // We need this first allocation to account for at least two entities for the list
            // to properly amortize the cost of the reallocations that will happen later.
            let layout = self
                .layout
                .layout_for_array(2)
                .expect("failed to allocate memroy");

            let data = unsafe { alloc::alloc::alloc(layout) };

            if data.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }

            self.data = unsafe { NonNull::new_unchecked(data) };
            self.cap = 2;

            return;
        }

        // This is guranteed not to overflow because we know that the length is strictly less
        // than the capacity.
        let amortized_new_cap = self.cap + self.cap / 2;

        // SAFETY:
        //  This is always valid because this is the layout that was originally used to allocate
        //  the memory in the first place.
        let layout = unsafe {
            Layout::from_size_align_unchecked(
                self.layout.size().wrapping_mul(self.cap),
                self.layout.align(),
            )
        };

        let new_size = amortized_new_cap
            .checked_mul(self.layout.size())
            .expect("failed to allocate memory");

        let new_data = unsafe { alloc::alloc::realloc(self.data.as_ptr(), layout, new_size) };

        if new_data.is_null() {
            let new_layout =
                unsafe { Layout::from_size_align_unchecked(new_size, self.layout.align()) };
            alloc::alloc::handle_alloc_error(new_layout);
        }

        self.data = unsafe { NonNull::new_unchecked(new_data) };
        self.cap = amortized_new_cap;
    }

    /// Pushes a new entity within the capacity of this list.
    ///
    /// # Safety
    ///
    /// - The length of the list must be *strictly* less than its capacity.
    ///
    /// - The provided `init` implementation must be associated with the same archetype as this
    ///   list.
    #[inline(always)]
    pub unsafe fn push_within_capacity_unchecked<E>(&mut self, init: E)
    where
        E: InitializeEntity,
    {
        // This can never overflow because we know that the length is strictly less than
        // the capacity, meaning that we were able to allocate that memory in the first place.
        self.get_unchecked(self.len).write(init);

        // This can never overflow because we know that the length is strictly less than
        // the capacity.
        self.len = self.len.wrapping_add(1);
    }

    /// Pushes a new entity within the capacity of this list.
    ///
    /// # Safety
    ///
    /// The provided `init` implementation must be associated with the same archetype as this
    /// list.
    #[inline]
    pub unsafe fn push<E>(&mut self, init: E)
    where
        E: InitializeEntity,
    {
        if self.len == self.cap {
            self.rallocate_for_push();
        }

        self.push_within_capacity_unchecked(init);
    }

    /// Removes the entity at the provided index from the list. Its slot is replaced by the last
    /// entity.
    ///
    /// # Safety
    ///
    /// `index` must be within the bounds of the list.
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> OwnedEntity {
        // Swap the entity with the last one.
        let removed_ptr = self.data.as_ptr().add(index * self.layout.size());
        let new_len = self.len - 1;
        let last_ptr = self.data.as_ptr().add(new_len * self.layout().size());

        // No need to swap anything, we're removing the last entity.
        if removed_ptr != last_ptr {
            core::ptr::swap_nonoverlapping(removed_ptr, last_ptr, self.layout.size());
        }

        self.len = new_len;

        // Creating an `OwnedEntity` ensures that the components will be properly
        // dropped.
        OwnedEntity::new(self.get_unchecked(new_len))
    }

    /// Returns an [`EntityPtr`] to the entity at the provided index.
    ///
    /// If `index` is less than `len()`, then the entity is initialized. Otherwise, it's
    /// uninitialized.
    ///
    /// # Safety
    ///
    /// `index` must be within the capacity of the list.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> EntityPtr {
        EntityPtr::new(
            &self.layout,
            self.data.as_ptr().add(index * self.layout.size()),
        )
    }

    /// Updates the length of the vector without checking anything.
    ///
    /// # Safety
    ///
    /// The vector must contain at least that many initialized entities, and the new length
    /// must be less than the current vector capacity.
    #[inline(always)]
    pub unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    /// Creates a slice of entities from this list.
    #[inline]
    pub fn as_slice(&self) -> EntitySlice {
        unsafe { EntitySlice::from_raw_parts(&self.layout, self.data.as_ptr(), self.len) }
    }

    /// Removes the last entity from the list, if any.
    #[inline]
    pub fn pop(&mut self) -> Option<OwnedEntity> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe { Some(OwnedEntity::new(self.get_unchecked(self.len))) }
    }

    /// Clears the list.
    #[inline]
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl Drop for EntityTable {
    fn drop(&mut self) {
        // Drop the components.
        self.clear();

        if self.layout.size() == 0 {
            // This is a zero-sized component, we don't need to deallocate any memory.
            return;
        }

        // SAFETY:
        //  This is always valid because this is the layout that was originally used to allocate
        //  the memory in the first place.
        let layout = unsafe {
            Layout::from_size_align_unchecked(
                self.layout.size().wrapping_mul(self.cap),
                self.layout.align(),
            )
        };

        unsafe { alloc::alloc::dealloc(self.data.as_ptr(), layout) }
    }
}
