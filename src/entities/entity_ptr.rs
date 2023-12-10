use super::component::{ComponentId, ComponentMeta};
use super::entity_layout::{EntityLayout, Field, InitializeEntity};
use super::Component;

/// An entity that is owned.
///
/// This pointer owns the entity and will automatically drop its elements when dropped.
pub struct OwnedEntity<'a>(EntityPtr<'a>);

impl<'a> OwnedEntity<'a> {
    /// Creates a new [`OwnedEntity`] instance.
    ///
    /// # Safety
    ///
    /// The ownership of the components of the entity is logically transferred to the created
    /// [`OwnedEntity`] instance, meaning that it will take care of dropping them when it
    /// is itself dropped.
    #[inline(always)]
    pub(crate) unsafe fn new(entity: EntityPtr<'a>) -> Self {
        Self(entity)
    }

    /// Returns the raw entity pointer that this [`OwnedEntity`] instance owns.
    #[inline(always)]
    pub fn as_ptr(&self) -> EntityPtr {
        self.0
    }

    /// Returns the inner [`EntityPtr`] instance, forgetting the ownership of the entity.
    #[inline(always)]
    pub fn forget(self) -> EntityPtr<'a> {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }
}

impl<'a> Drop for OwnedEntity<'a> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.as_ptr().drop_in_place() };
    }
}

/// A raw entity pointer.
#[derive(Clone, Copy)]
pub struct EntityPtr<'a> {
    /// The layout of the entity.
    ///
    /// The lifetime of this reference is also the lifetime of the memory that holds the entity.
    layout: &'a EntityLayout,
    /// A pointer to the entity.
    data: *mut u8,
}

impl<'a> EntityPtr<'a> {
    /// Creates a new [`EntityPtr`] instance.
    ///
    /// # Safety
    ///
    /// `data` must point to a valid initialized entity with the provided layout. The memory
    /// that the entity lives in must live for at least the lifetime `'a`.
    #[inline(always)]
    pub(crate) unsafe fn new(layout: &'a EntityLayout, data: *mut u8) -> Self {
        Self { layout, data }
    }

    /// Returns the layout of the entity.
    #[inline(always)]
    pub fn layout(self) -> &'a EntityLayout {
        self.layout
    }

    /// Returns an iterator over the raw components of the entity.
    pub fn components(self) -> impl 'a + Iterator<Item = (*mut u8, &'a ComponentMeta)> {
        self.layout
            .components()
            .map(move |field| (unsafe { self.data.add(field.offset) }, &field.meta))
    }

    /// Returns whether the entity has a component of the provided type.
    #[inline]
    pub fn has_component(self, id: ComponentId) -> bool {
        self.layout.has_component(id)
    }

    /// Returns the raw pointer to the entity.
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.data
    }

    /// Drops the components of this entity in-place.
    ///
    /// # Safety
    ///
    /// It must be safe to access the components of this entity mutably. The components
    /// must be properly initialized. After this function returns, those values must never
    /// be used again.
    pub unsafe fn drop_in_place(self) {
        self.components()
            .for_each(|(ptr, meta)| meta.drop_in_place(ptr));
    }

    /// Returns a pointer to the component of the provided type, or a null pointer if the component
    /// is not present in the entity.
    #[inline]
    pub fn get_field(self, id: ComponentId) -> Option<(*mut u8, &'a Field)> {
        self.layout
            .field_of(id)
            .map(move |field| (unsafe { self.data.add(field.offset) }, field))
    }

    /// Returns a pointer to the component of the provided type.
    ///
    /// # Safety
    ///
    /// This function assumes that the component is present in the entity.
    pub unsafe fn get_field_unchecked(self, id: ComponentId) -> (*mut u8, &'a Field) {
        let field = match self.layout.field_of(id) {
            Some(field) => field,
            None => {
                debug_assert!(false);
                unsafe { core::hint::unreachable_unchecked() };
            }
        };

        let ptr = unsafe { self.data.add(field.offset) };
        (ptr, &field)
    }

    /// Returns a pointer to the component of the provided type, or a null pointer if
    /// the component is not present in the entity.
    #[inline(always)]
    pub fn get_raw<T: Component>(self) -> *mut T {
        match self.get_field(ComponentId::of::<T>()) {
            Some((ptr, _)) => ptr.cast(),
            None => core::ptr::null_mut(),
        }
    }

    /// Initializes the components of this entity using the provided [`InitializeEntity`]
    /// implementation.
    ///
    /// # Safety
    ///
    /// This entity must include at least the components initialized by the provided
    /// [`InitializeEntity`] implementation.
    pub unsafe fn write<E>(self, init: E)
    where
        E: InitializeEntity,
    {
        init.write_components(|id, src| {
            let (dst, field) = self.get_field_unchecked(id);
            core::ptr::copy_nonoverlapping(src, dst, field.meta.layout().size());
        });
    }
}

impl PartialEq for EntityPtr<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.data, other.data)
    }
}

impl Eq for EntityPtr<'_> {}

/// A slice of entities with a same [`EntityLayout`].
#[derive(Clone, Copy)]
pub struct EntitySlice<'a> {
    /// The layout of the entities.
    layout: &'a EntityLayout,

    /// A pointer to the first entity in the slice.
    data: *mut u8,

    /// The number of entities in the slice.
    len: usize,
}

impl<'a> EntitySlice<'a> {
    /// Creates a new [`EntitySlice`] instance.
    ///
    /// # Safety
    ///
    /// `data` must point to a valid slice of entities with the provided layout. The memory
    /// that the entities live in must live for at least the lifetime `'a`.
    #[inline(always)]
    pub unsafe fn from_raw_parts(layout: &'a EntityLayout, data: *mut u8, len: usize) -> Self {
        Self { layout, data, len }
    }

    /// Returns a pointer to the entity at `index`.
    ///
    /// # Safety
    ///
    /// `index` must be within the bounds of the slice.
    #[inline]
    pub unsafe fn get_unchecked(self, index: usize) -> EntityPtr<'a> {
        // SAFETY:
        //  The index is within the bounds of the slice.
        EntityPtr::new(self.layout, self.data.add(index * self.layout.size()))
    }

    /// Returns a pointer to the entity at `index`, or `None` if `index` is out of bounds.
    #[inline]
    pub fn get(self, index: usize) -> Option<EntityPtr<'a>> {
        if index < self.len {
            // SAFETY:
            //  The index is within the bounds of the slice.
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    /// Returns the layout of the entities in the slice.
    #[inline(always)]
    pub fn layout(&self) -> &'a EntityLayout {
        self.layout
    }

    /// Returns the length of the slice.
    #[inline(always)]
    pub fn len(self) -> usize {
        self.len
    }

    /// Returns whether the slice is empty.
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}
