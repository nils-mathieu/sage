use crate::Uuid;

/// A sorted list of distinct [`Uuid`]s representing the components that are part of an archetype
/// storage.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeComponents([Uuid]);

impl ArchetypeComponents {
    /// Creates a new [`ArchetypeComponents`] from a slice of [`Uuid`]s.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided slice is sorted and contains no duplicates.
    #[inline(always)]
    pub unsafe fn from_boxed_slice_unchecked(slice: Box<[Uuid]>) -> Box<Self> {
        unsafe { Box::from_raw(Box::into_raw(slice) as *mut Self) }
    }

    /// Returns a slice of the [`Uuid`]s stored in this [`ArchetypeComponents`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the returned slice is sorted and contains no duplicates.
    #[inline(always)]
    pub unsafe fn from_slice_unchecked(slice: &[Uuid]) -> &Self {
        unsafe { &*(slice as *const [Uuid] as *const Self) }
    }

    /// Returns the list of [`Uuid`]s stored in this [`ArchetypeComponents`] instance.
    #[inline(always)]
    pub fn as_uuids(&self) -> &[Uuid] {
        &self.0
    }
}

impl ToOwned for ArchetypeComponents {
    type Owned = Box<ArchetypeComponents>;

    fn to_owned(&self) -> Self::Owned {
        unsafe { Self::from_boxed_slice_unchecked(Box::from(self.as_uuids())) }
    }
}

impl From<&'_ ArchetypeComponents> for Box<ArchetypeComponents> {
    #[inline(always)]
    fn from(value: &ArchetypeComponents) -> Self {
        value.to_owned()
    }
}

/// A sorted list of distinct [`Uuid`]s representing the components that are part of an archetype
/// storage.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct StaticArchetypeComponents<const N: usize>([Uuid; N]);

impl<const N: usize> StaticArchetypeComponents<N> {
    /// Creates a new [`StaticArchetypeComponents`] from an array of [`Uuid`]s.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided array is sorted and contains no duplicates.
    #[inline]
    pub const unsafe fn new_unchecked(array: [Uuid; N]) -> Self {
        Self(array)
    }
}

impl<const N: usize> AsRef<ArchetypeComponents> for StaticArchetypeComponents<N> {
    #[inline(always)]
    fn as_ref(&self) -> &ArchetypeComponents {
        unsafe { ArchetypeComponents::from_slice_unchecked(&self.0) }
    }
}

impl<const N: usize> From<StaticArchetypeComponents<N>> for Box<ArchetypeComponents> {
    #[inline]
    fn from(value: StaticArchetypeComponents<N>) -> Self {
        value.as_ref().to_owned()
    }
}
