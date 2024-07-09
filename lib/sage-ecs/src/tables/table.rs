use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::{sparse_set::SparseSet, tables::column::Column};

/// Stores a collection with a specific set of components.
pub struct Table<E> {
    /// The columns that are responsible for storing entity components in this table.
    columns: SparseSet<Column, u8>,
    /// Some metadata associated with the entities in the table.
    metadata: Vec<E>,
}

impl<E> Table<E> {
    /// Creates a new [`Table`] instance with no entities.
    pub const fn new() -> Self {
        Self {
            columns: SparseSet::new(),
            metadata: Vec::new(),
        }
    }

    /// Returns the number of entities in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Returns `true` if the table contains no entities.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Returns a reference to the metadata of the entities in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn metadata(&self) -> &[E] {
        &self.metadata
    }

    /// Returns a mutable reference to the metadata of the entities in the table.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn metadata_mut(&mut self) -> &mut [E] {
        &mut self.metadata
    }

    /// Reserves capacity for at least `additional` more entities to be inserted in the table
    /// without reallocating.
    pub fn reserve(&mut self, additional: usize) {
        self.metadata.reserve(additional);
        self.columns
            .dense_mut()
            .iter_mut()
            .for_each(|c| c.reserve(additional));
    }

    /// Returns the spare capacity of the metadata vector.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn metadata_spare_capacity(&mut self) -> &mut [MaybeUninit<E>] {
        self.metadata.spare_capacity_mut()
    }

    /// Assumes that `additional` entities have been initialized.
    ///
    /// # Safety
    ///
    /// The caller must make sure that `additional` components & metadata have been initialized
    /// before calling this method.
    pub unsafe fn assume_init_push(&mut self, additional: usize) {
        unsafe {
            self.metadata
                .set_len(self.metadata.len().unchecked_add(additional));
            self.columns
                .dense_mut()
                .iter_mut()
                .for_each(|c| c.assume_init_push(additional));
        }
    }
}

impl<E> Default for Table<E> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}
