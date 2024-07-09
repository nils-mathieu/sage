//! Raw storage for arbitrary entities.

use alloc::vec::Vec;

mod column;
use crate::component::Registry;

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

    /// Reserves memory for additional entities in the provided table.
    ///
    /// # Panics
    ///
    /// This function panics if the memory cannot be allocated.
    ///
    /// # Safety
    ///
    /// The provided `table` ID must be valid. Id 0 is always valid and refers to the
    /// table with no components.
    pub unsafe fn reserve(&mut self, table: TableId, additional: usize) {
        debug_assert!(table < self.tables.len());
        let table = unsafe { self.tables.get_unchecked_mut(table) };
        table.reserve(additional);
    }

    /// Spawns an empty entity.
    pub fn spawn_empty(&mut self, metadata: E) -> EntityLocation {
        unsafe {
            // The ID 0 is reserved for the table with no components.
            let table_id = 0;

            let table = self.tables.get_unchecked_mut(table_id);
            let table_row = table.len();
            table.push(metadata, ());

            EntityLocation {
                table_id,
                table_row,
            }
        }
    }
}

impl<E> Default for Tables<E> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}
