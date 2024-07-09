//! Raw storage for arbitrary entities.

use alloc::vec::Vec;

mod column;
pub use self::column::*;

mod table;
pub use self::table::*;

/// The ID of a table in the [`Tables`] storage.
pub type TableId = usize;

/// The row number of an entity within a table.
pub type TableRow = usize;

/// Represents the location of an entity living in an [`Tables`] collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityLocation {
    /// The ID of the table that contains the entity.
    pub table_id: TableId,
    /// The row number of the entity within the table.
    pub table_row: TableRow,
}

/// Allows storing entities with arbitrary set of components.
///
/// # Remarks
///
/// Unlike the more flexible [`UnsafeWorld`] type, this type does not provide stability for
/// spawned entities. This means that entity locations may change when the set of components
/// of an entity is modified.
///
/// # Generic parameters
///
/// The `E` generic parameter is the type of metadata that is kept on a per-entity basis.
pub struct Tables<E> {
    /// The storages that contain entities, separated by their archetype (the unique set of
    /// components that they contain).
    ///
    /// This vector is indexed by the [`TableId`] of the entity.
    ///
    /// The first element of this vector is always the archetype with no components.
    tables: Vec<Table<E>>,
}

impl<E> Tables<E> {
    /// Creates a new [`Tables`] instance with no entities.
    pub fn new() -> Self {
        Self {
            tables: alloc::vec![Table::new()],
        }
    }

    /// Spawns an empty entity.
    pub fn spawn_empty(&mut self, metadata: E) -> EntityLocation {
        // The ID 0 is reserved for the table with no components.
        let table_id = 0;

        let table = unsafe { self.tables.get_unchecked_mut(table_id) };
        table.reserve(1);

        let table_row = table.len();

        unsafe {
            table
                .metadata_spare_capacity()
                .get_unchecked_mut(0)
                .write(metadata);

            // SAFETY: We properly initialized a single entity. There was no components to
            // initialize because the entity is empty.
            table.assume_init_push(1);
        }

        EntityLocation {
            table_id,
            table_row,
        }
    }
}

impl<E> Default for Tables<E> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}
