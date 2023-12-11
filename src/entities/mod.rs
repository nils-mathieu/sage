use core::hash::{BuildHasher, Hash, Hasher};

use hashbrown::HashMap;
use rustc_hash::FxHasher;

use alloc::boxed::Box;
use alloc::vec::Vec;

mod entity_allocator;
mod entity_table;

mod archetype;
pub use archetype::*;

mod component;
pub use component::*;

mod entity_ptr;
pub use entity_ptr::*;

pub mod add_components;
pub mod entity_layout;
pub mod remove_components;

use self::entity_allocator::EntityAllocator;
use self::entity_layout::{
    Components, EntityLayout, InitializeEntity, IntoEntityLayout, StaticComponents,
};
use self::entity_table::EntityTable;

pub use entity_allocator::Entity;

/// The location of an entity within an [`Entities`] collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityLocation {
    /// The index of the [`EntityTable`] that contains the entity.
    table: usize,
    /// The index of the entity within the [`EntityTable`] associated with the archetype.
    index: usize,
}

/// The metadata stored about a specific archetype.
struct TableEntry {
    /// The table that stores the entities with this archetype.
    table: EntityTable,
    /// The list of entity indices that have that archetype.
    ///
    /// This is used when removing an entity from the collection. It allows us to quickly find
    /// the index of the removed entity to update its location.
    ///
    /// The entities within this list are stored in the order in which they appear in the
    /// corresponding [`EntityTable`].
    entities: Vec<Entity>,
    /// The archetype associated with this entry.
    archetype: Box<Archetype>,
}

/// An implementation of [`BuildHasher`] that creates an instance of [`FxHasher`].
struct BuildFxHasher;

impl BuildHasher for BuildFxHasher {
    type Hasher = FxHasher;

    #[inline(always)]
    fn build_hasher(&self) -> Self::Hasher {
        FxHasher::default()
    }
}

/// The map that's responsible for translating an [`Archetype`] instance to the index of the
/// corresponding [`EntityTable`] instance.
type Archetypes = HashMap<Box<Archetype>, usize, BuildFxHasher>;

/// A collection of entities.
///
/// # Mutable Access
///
/// This type considers all of the components it stores to be *inside of an `UnsafeCell`. It is the
/// responsability of the user of the collection to correctly access the components mutably when
/// it's allowed to do so.
pub struct Entities {
    /// The allocator used to create new [`Entity`] instances.
    allocator: EntityAllocator<EntityLocation>,
    /// A map that translates an entity archetype to the index of the corresponding [`EntityTable`]
    /// instance.
    archetypes: Archetypes,
    /// The list of all archetype entries.
    ///
    /// Those entries include the actual entity tables, as well as other bookkeeping information.
    tables: Vec<TableEntry>,
}

impl Entities {
    /// Creates a new [`Entities`] instance.
    #[inline]
    pub const fn new() -> Self {
        Self {
            allocator: EntityAllocator::new(),
            archetypes: Archetypes::with_hasher(BuildFxHasher),
            tables: Vec::new(),
        }
    }

    /// Returns the table that's associated with the provided archetype.
    ///
    /// If the table does not exist yet, it is created.
    #[inline]
    fn get_table_for<L: IntoEntityLayout>(&mut self, layout: L) -> usize {
        use hashbrown::hash_map::RawEntryMut;

        let archetype = layout.archetype();
        let archetype_ref = archetype.as_ref();
        let archetype_hash = {
            let mut hasher = FxHasher::default();
            archetype_ref.hash(&mut hasher);
            hasher.finish()
        };

        match self
            .archetypes
            .raw_entry_mut()
            .from_key_hashed_nocheck(archetype_hash, archetype_ref)
        {
            RawEntryMut::Occupied(e) => *e.get(),
            RawEntryMut::Vacant(e) => {
                // The archetype has never been seen before, we have to
                // create a new entry for it.
                let table_idx = self.tables.len();

                e.insert_hashed_nocheck(archetype_hash, archetype_ref.clone_boxed(), table_idx);

                self.tables.push(TableEntry {
                    archetype: archetype.into(),
                    entities: Vec::new(),
                    table: EntityTable::new(layout.into_layout()),
                });

                table_idx
            }
        }
    }

    /// An (eventually dynamic) collection of components.
    pub fn spawn<C>(&mut self, components: C) -> Entity
    where
        C: Components,
    {
        let table_index = self.get_table_for(components.archetype());
        let table = unsafe { self.tables.get_unchecked_mut(table_index) };

        let entity = self.allocator.allocate(EntityLocation {
            index: table.table.len(),
            table: table_index,
        });

        // SAFETY:
        //  The table is suitable for storing the provided components.
        unsafe { table.table.push(components) };
        table.entities.push(entity);

        entity
    }

    /// Spawns a bunch of entities, returning their indices within the collection.
    ///
    /// # Note
    ///
    /// If the returned iterator is not completely consumed, the remaining entities will *not*
    /// be inserted into the collection!
    pub fn spawn_batch<I>(&mut self, batch: I) -> SpawnBatch<I::IntoIter>
    where
        I: IntoIterator,
        I::Item: StaticComponents,
    {
        let table_index = self.get_table_for(<I::Item as StaticComponents>::archetype());
        let table = unsafe { self.tables.get_unchecked_mut(table_index) };

        SpawnBatch {
            iter: batch.into_iter(),
            allocator: &mut self.allocator,
            table_index,
            table,
        }
    }

    /// Returns whether the provided entity is live or not.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.allocator.is_live(entity)
    }

    /// Edits an existing entity using the provided [`EditEntity`] implementation.
    ///
    /// # Safety
    ///
    /// The provided entity must be live.
    pub unsafe fn edit<E>(&mut self, entity: u32, edit: E) -> E::Output
    where
        E: EditEntity,
    {
        let old_location = *self.allocator.get_unchecked(entity);

        // We can't use something like `get_unchecked_mut` here because that would borrow
        // the whole vector.
        let old_table = &mut *self.tables.as_mut_ptr().add(old_location.table);

        let new_archetype =
            edit.new_archetype(old_table.archetype.as_ref(), old_table.table.layout());
        let new_table_index = self.get_table_for(new_archetype);

        if new_table_index == old_location.table {
            // The entity does not actually change archetype.
            edit.edit_in_place(old_table.table.get_unchecked(old_location.index))
        } else {
            // It's safe to access the other table mutably because we just ensured
            // that it's not the same as the current table.
            // Once again, it's not possible to use `get_unchecked_mut` here because
            // that would borrow the whole vector.
            let new_table = &mut *self.tables.as_mut_ptr().add(new_table_index);

            let old = old_table.table.swap_remove_unchecked(old_location.index);
            let old_entity = swap_remove_unchecked(&mut old_table.entities, old_location.index);

            // Fix the location of the removed entity.
            self.allocator.get_unchecked_mut(old_entity.index()).index = old_location.index;

            let new_index = new_table.table.len();
            new_table.table.reserve_one();
            let new = new_table.table.get_unchecked(new_index);

            let output = edit.edit(old.forget(), new);

            // At this point, `new` is initialized, and `old` is uninitialized.
            new_table.table.set_len(new_table.table.len() + 1);

            // Reroute the entity to the new location.
            *self.allocator.get_unchecked_mut(entity) = EntityLocation {
                index: new_index,
                table: new_table_index,
            };

            output
        }
    }

    /// Despawns the provided entity from this collection.
    ///
    /// # Safety
    ///
    /// The provided entity must be live.
    pub unsafe fn despawn(&mut self, entity: u32) -> OwnedEntity {
        let location = self.allocator.deallocate_unchecked(entity);
        let table = self.tables.get_unchecked_mut(location.table);

        let removed = table.table.swap_remove_unchecked(location.index);
        let removed_entity = swap_remove_unchecked(&mut table.entities, location.index);

        // Fix the location of the moved entity (the entity that was swapped with the
        // removed entity).
        self.allocator
            .get_unchecked_mut(removed_entity.index())
            .index = location.index;

        removed
    }

    /// Returns a raw entity pointer to the provided entity.
    ///
    /// # Safety
    ///
    /// The provided entity must be live.
    #[inline]
    pub unsafe fn get(&self, entity: u32) -> EntityPtr {
        let location = self.allocator.get_unchecked(entity);
        self.tables
            .get_unchecked(location.table)
            .table
            .get_unchecked(location.index)
    }

    /// Returns an iterator over the individual entity tables.
    ///
    /// This is useful for iterating over all entities in the collection.
    #[inline]
    pub fn tables(&self) -> Tables<'_> {
        Tables(self.tables.iter())
    }
}

/// An iterator over the individual entity tables of an [`Entities`] collection.
pub struct Tables<'a>(core::slice::Iter<'a, TableEntry>);

impl<'a> Iterator for Tables<'a> {
    type Item = (&'a [Entity], EntitySlice<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|table| (table.entities.as_slice(), table.table.as_slice()))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// An iterator that inserts entity of a given type into an [`Entities`] collection.
pub struct SpawnBatch<'a, I> {
    /// The iterator that yields the entities to insert.
    ///
    /// Those entities must be valid for the associated [`EntityTable`].
    iter: I,
    /// The allocator that will be used to create the entities.
    allocator: &'a mut EntityAllocator<EntityLocation>,
    /// the index of the table we're currently inserting entities into.
    table_index: usize,
    /// The archetype entry that will store the created entities.
    table: &'a mut TableEntry,
}

impl<I> Iterator for SpawnBatch<'_, I>
where
    I: Iterator,
    I::Item: InitializeEntity,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let init = self.iter.next()?;
        let entity = self.allocator.allocate(EntityLocation {
            index: self.table.table.len(),
            table: self.table_index,
        });

        unsafe { self.table.table.push(init) };
        self.table.entities.push(entity);

        Some(entity)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// Removes an element from a [`Vec<T>`] without checking whether the index is valid.
///
/// # Safety
///
/// The provided index must be valid for the provided vector.
unsafe fn swap_remove_unchecked<T>(v: &mut Vec<T>, index: usize) -> T {
    let value = v.as_ptr().add(index).read();

    let new_len = v.len().wrapping_sub(1);

    v.as_mut_ptr()
        .add(index)
        .write(v.as_ptr().add(new_len).read());
    v.set_len(new_len);

    value
}

/// A trait that can be used to modify the components of an entity.
///
/// The two canonical implementations of this trait are [`RemoveComponents`] and [`AddComponents`].
///
/// # Safety
///
/// - [`edit`] must properly initialize the components of `new`, and deinitialize the components
///   of `old`.
///
/// [`edit`]: Self::edit
pub unsafe trait EditEntity {
    /// The return type of the [`new_archetype`] function.
    type Archetype<'a>: IntoEntityLayout
    where
        Self: 'a;

    /// Given a reference to the current archetype of the entity, returns the new archetype that
    /// it's going to have after the edit.
    ///
    /// # Safety
    ///
    /// `archetype` and `layout` must be coherent with one another.
    unsafe fn new_archetype<'a>(
        &'a self,
        archetype: &'a Archetype,
        layout: &'a EntityLayout,
    ) -> Self::Archetype<'a>;

    /// The output of the edition.
    type Output;

    /// Applies the edit to the provided entity, in place because the new archetype is actually
    /// the same as the old one.
    ///
    /// # Safety
    ///
    /// - Accessing the components of the provided entity mutably must be safe.
    ///
    /// - The archetype of the entity, when passed through `new_archetype`, must return
    /// the exact same archetype.
    unsafe fn edit_in_place(self, entity: EntityPtr) -> Self::Output;

    /// Applies the edit to the entity by deinitializing the components of the old archetype and
    /// initializing the components of the new one.
    ///
    /// Upon returning this function must ensure that `new` is properly initialized, and that
    /// `old` is properly deinitialized.
    ///
    /// # Safety
    ///
    /// - `new` must be an entity with the archetype returned by [`new_archetype`] when given
    ///   the archetype of `old`.
    ///
    /// - `old` and `new` must be accessible mutably.
    ///
    /// [`new_archetype`]: Self::new_archetype
    unsafe fn edit(self, old: EntityPtr, new: EntityPtr) -> Self::Output;
}
