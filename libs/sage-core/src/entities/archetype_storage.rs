use crate::{
    Uuid,
    entities::{ComponentLayout, ComponentList, EntityIndex, component_vec::ComponentVec},
};

/// A collection of entities that all share the same set of components.
pub struct ArchetypeStorage {
    /// The IDs of the entities stored in this collection.
    ids: Vec<EntityIndex>,
    /// The components stored in this collection.
    columns: hashbrown::HashMap<Uuid, ComponentVec, foldhash::fast::FixedState>,
}

impl ArchetypeStorage {
    /// Creates a new, empty [`ArchetypeStorage`] responsible for storing the provided components.
    pub fn new(components: impl IntoIterator<Item = (Uuid, ComponentLayout)>) -> Self {
        Self {
            ids: Vec::new(),
            columns: components
                .into_iter()
                .map(|(id, layout)| (id, ComponentVec::new(layout)))
                .collect(),
        }
    }

    /// Returns the total number of entities stored in this collection.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns the number of entities that this collection can store without reallocating.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Reserves the necessary memory to push a new entity into this collection.
    pub fn reserve_one(&mut self) {
        self.ids.reserve(1);
        for column in self.columns.values_mut() {
            column.reserve_one();
        }
    }

    /// Pushes a new entity into this collection.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`ComponentList`] initializes exactly the
    /// components that this collection stores.
    ///
    /// The caller must ensure that the [`reserve_one`](ArchetypeStorage::reserve_one) method has
    /// been called previously to make sure that the collection has enough capacity to store the
    /// new entity.
    pub unsafe fn push_assume_capacity(
        &mut self,
        entity_index: EntityIndex,
        components: impl ComponentList,
    ) {
        unsafe { push_assume_capacity(&mut self.ids, entity_index) };
        components.write(|id, src| unsafe {
            let column = self.columns.get_mut(&id).unwrap_unchecked();
            column.push_assume_capacity(src);
        });
    }

    /// Swap removes the entity at the provided index.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided index is within bounds.
    ///
    /// # Returns
    ///
    /// This functionr returns the index of the entity that was swap-removed.
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> EntityIndex {
        let entity_index = unsafe { swap_remove_unchecked(&mut self.ids, index) };
        for column in self.columns.values_mut() {
            unsafe { column.swap_remove_unchecked(index) };
        }
        entity_index
    }

    /// Returns whether this collection stores the components with the provided UUID or not.
    #[inline]
    pub fn has_component(&self, uuid: Uuid) -> bool {
        self.columns.contains_key(&uuid)
    }

    /// Returns a pointer to the column responsible for storing the components with the provided
    /// UUID.
    #[inline]
    pub fn get_column(&self, uuid: Uuid) -> Option<&ComponentVec> {
        self.columns.get(&uuid)
    }

    /// Returns a mutable pointer to the column responsible for storing the components with the
    /// provided UUID.
    #[inline]
    pub fn get_column_mut(&mut self, uuid: Uuid) -> Option<&mut ComponentVec> {
        self.columns.get_mut(&uuid)
    }

    /// Returns the [`EntityIndex`] of the entities stored in this collection.
    #[inline(always)]
    pub fn entity_indices(&self) -> &[EntityIndex] {
        &self.ids
    }
}

/// Pushes a value into the provided vector without checking whether there is enough capacity
/// for it or not.
///
/// # Safety
///
/// The caller must ensure that the length of the vector is strictly less than its capacity.
unsafe fn push_assume_capacity<T>(v: &mut Vec<T>, val: T) {
    unsafe {
        let len = v.len();
        v.as_mut_ptr().add(len).write(val);
        v.set_len(len.unchecked_add(1));
    }
}

/// Performs a swap-remove operation on the provided vector without checking whether
/// the index is within bounds or not.
///
/// # Safety
///
/// The caller must ensure that the index is within bounds.
unsafe fn swap_remove_unchecked<T>(v: &mut Vec<T>, index: usize) -> T {
    unsafe {
        let new_len = v.len().unchecked_sub(1);
        let value = std::ptr::read(v.as_ptr().add(index));
        let base_ptr = v.as_mut_ptr();
        std::ptr::copy(base_ptr.add(new_len), base_ptr.add(index), 1);
        v.set_len(new_len);
        value
    }
}
