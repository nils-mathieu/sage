use crate::{
    entity::{Entity, EntityAllocator},
    tables::{EntityLocation, Tables},
};

/// A collection of entities.
pub struct UnsafeWorld {
    tables: Tables<Entity>,
    entity_allocator: EntityAllocator<EntityLocation>,
}

impl UnsafeWorld {
    /// Creates a new empty [`UnsafeWorld`].
    pub fn new() -> Self {
        Self {
            tables: Tables::new(),
            entity_allocator: EntityAllocator::new(),
        }
    }

    /// Reserves an empty entity.
    ///
    /// Unlike the regular [`spawn`] method, this function does not require the [`UnsafeWorld`]
    /// to be borrowed exclusively. This allows for spawning entities in the middle of iterating
    /// over entities.
    ///
    /// Note that the entity won't actually be spawned until the next call to [`flush`] (or any
    /// function that calls it for you).
    ///
    /// [`spawn`]: UnsafeWorld::spawn
    /// [`flush`]: UnsafeWorld::flush
    ///
    /// # Returns
    ///
    /// This function returns the entity that was reserved.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn reserve_one(&self) -> Entity {
        self.entity_allocator.reserve_one()
    }

    /// Spawns a new enity with the given components.
    pub fn spawn(&mut self) {
        self.flush();
        todo!();
    }

    /// Flushes reserved entities.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn flush(&mut self) {
        #[inline(never)]
        #[cold]
        fn do_flush(this: &mut UnsafeWorld) {
            // SAFETY: Table ID 0 is always valid.
            unsafe { this.tables.reserve(0, this.entity_allocator.reserved()) };
            this.entity_allocator
                .flush(|entity| this.tables.spawn_empty(entity));
        }

        if self.entity_allocator.needs_flush() {
            do_flush(self);
        }
    }
}

impl Default for UnsafeWorld {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}
