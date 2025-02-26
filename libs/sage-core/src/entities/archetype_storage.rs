use {
    super::ArchetypeComponents,
    crate::{
        OpaquePtr, Uuid,
        entities::{ComponentInfo, ComponentList, EntityIndex, component_vec::ComponentVec},
    },
};

/// The row of an entity within an [`ArchetypeStorage`].
pub type EntityRow = usize;

/// A collection of entities that all share the same set of components.
pub struct ArchetypeStorage {
    /// The archetype of the entities stored in this collection.
    components: Box<ArchetypeComponents>,
    /// The IDs of the entities stored in this collection.
    ids: Vec<EntityIndex>,
    /// The components stored in this collection.
    columns: hashbrown::HashMap<Uuid, ComponentVec, foldhash::fast::FixedState>,
}

impl ArchetypeStorage {
    /// Creates a new, empty [`ArchetypeStorage`] responsible for storing the provided components.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided iterator returns distinct [`ComponentInfo`]
    /// instances, sorted by their UUIDs.
    pub fn new(info: impl IntoIterator<Item = &'static ComponentInfo>) -> Self {
        let iter = info.into_iter();
        let count = iter.size_hint().0;

        let mut columns = hashbrown::HashMap::with_capacity_and_hasher(count, Default::default());
        let mut components = Vec::with_capacity(count);

        for info in iter {
            unsafe {
                columns.insert_unique_unchecked(info.uuid, ComponentVec::new(info));
                push_assume_capacity(&mut components, info.uuid);
            }
        }

        let components = unsafe {
            ArchetypeComponents::from_boxed_slice_unchecked(components.into_boxed_slice())
        };

        Self {
            ids: Vec::new(),
            columns,
            components,
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

    /// Returns the [`ArchetypeComponents`] associated with the entities stored in this collection.
    #[inline(always)]
    pub fn archetype_components(&self) -> &ArchetypeComponents {
        &self.components
    }

    /// Reserves the necessary memory to push a new entity into this collection.
    pub fn reserve_one(&mut self) {
        self.ids.reserve(1);
        for column in self.columns.values_mut() {
            column.reserve_one();
        }
    }

    /// Reserves the necessary memory to push the requested number of entities
    /// into the collection without reallocation.
    pub fn reserve(&mut self, additional: usize) {
        self.ids.reserve(additional);
        for column in self.columns.values_mut() {
            column.reserve(additional);
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
        components.write(&mut |id, src| unsafe {
            let column = self.columns.get_mut(&id).unwrap_unchecked();
            column.push_assume_capacity(src);
        });
    }

    /// Assumes that an entity has been pushed to the end of the storage.
    ///
    /// # Safety
    ///
    /// The entity must really have been pushed to the end of the storage.
    pub fn assume_pushed(&mut self, entity_index: EntityIndex) {
        unsafe {
            push_assume_capacity(&mut self.ids, entity_index);
            for column in self.columns.values_mut() {
                column.set_len(column.len().unchecked_add(1));
            }
        }
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
    pub unsafe fn swap_remove_unchecked(&mut self, index: EntityRow) -> EntityIndex {
        let entity_index = unsafe { swap_remove_unchecked(&mut self.ids, index) };
        for column in self.columns.values_mut() {
            unsafe { column.swap_remove_unchecked(index) };
        }
        entity_index
    }

    /// Swap-removes the entity at the provided index, assuming it has already been moved
    /// out/dropped.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `index` is within the bounds of the storage.
    ///
    /// # Returns
    ///
    /// This function returns the index of the entity that was swap-removed.
    pub unsafe fn swap_remove_unchecked_no_drop(&mut self, index: usize) -> EntityIndex {
        let entity_index = unsafe { swap_remove_unchecked(&mut self.ids, index) };
        for column in self.columns.values_mut() {
            unsafe { column.swap_remove_unchecked_no_drop(index) };
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

    /// Returns an iterator over the columns stored in this collection.
    pub fn columns(&self) -> impl Iterator<Item = (Uuid, &ComponentVec)> {
        self.columns.iter().map(|(uuid, column)| (*uuid, column))
    }

    /// Returns an iterator over the columns stored in this collection.
    pub fn columns_mut(&mut self) -> impl Iterator<Item = (Uuid, &mut ComponentVec)> {
        self.columns
            .iter_mut()
            .map(|(uuid, column)| (*uuid, column))
    }

    /// Returns an [`ArchetypeStorageRef`] to the entity at the provided index.
    #[inline]
    pub fn get(&self, index: usize) -> ArchetypeStorageRef {
        ArchetypeStorageRef {
            storage: self,
            index,
        }
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
unsafe fn swap_remove_unchecked<T>(v: &mut Vec<T>, index: EntityRow) -> T {
    unsafe {
        let new_len = v.len().unchecked_sub(1);
        let value = std::ptr::read(v.as_ptr().add(index));
        let base_ptr = v.as_mut_ptr();
        std::ptr::copy(base_ptr.add(new_len), base_ptr.add(index), 1);
        v.set_len(new_len);
        value
    }
}

/// A view into a specific entity within an [`ArchetypeStorage`].
pub struct ArchetypeStorageRef<'a> {
    /// The referenced storage.
    storage: &'a ArchetypeStorage,
    /// The index of the entity within the storage.
    index: usize,
}

impl ArchetypeStorageRef<'_> {
    /// Returns information about the component associated with the provided component UUID.
    ///
    /// Only works when the component is part of the associated storage.
    pub fn component_info(&self, uuid: Uuid) -> Option<&'static ComponentInfo> {
        self.storage.columns.get(&uuid).map(|x| x.component_info())
    }

    /// Returns the component associated with the provided component UUID.
    pub fn get_raw(&self, uuid: Uuid) -> Option<OpaquePtr> {
        self.storage
            .columns
            .get(&uuid)
            .map(|x| unsafe { x.get_unchecked(self.index) })
    }

    /// Returns the component associated with the provided component UUID along with its
    /// associated component information.
    pub fn get_raw_and_info(&self, uuid: Uuid) -> Option<(OpaquePtr, &'static ComponentInfo)> {
        self.storage
            .columns
            .get(&uuid)
            .map(|x| (unsafe { x.get_unchecked(self.index) }, x.component_info()))
    }

    /// Returns an iterator over the components that are part of the referenced entity.
    pub fn raw_components(
        &self,
    ) -> impl Iterator<Item = (Uuid, &'static ComponentInfo, OpaquePtr)> + '_ {
        self.storage
            .columns
            .iter()
            .map(move |(uuid, column)| unsafe {
                (
                    *uuid,
                    column.component_info(),
                    column.get_unchecked(self.index),
                )
            })
    }
}
