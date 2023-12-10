use alloc::boxed::Box;

use super::component::ComponentId;

/// Represents the archetype of an entity.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Archetype([ComponentId]);

impl Archetype {
    /// Creates a new [`Archetype`] instance from the provided list of component types.
    ///
    /// # Safety
    ///
    /// The provided type IDs must be stored in ascending order, and they must be unique.
    #[inline(always)]
    pub unsafe fn new_boxed(ids: Box<[ComponentId]>) -> Box<Self> {
        unsafe { Box::from_raw(Box::into_raw(ids) as *mut Self) }
    }

    /// Creates a new [`Archetype`] instance from the provided list of component types.
    ///
    /// # Safety
    ///
    /// The provided type IDs must be stored in descending order.
    #[inline(always)]
    pub unsafe fn new_ref(ids: &[ComponentId]) -> &Self {
        unsafe { &*(ids as *const [ComponentId] as *const Self) }
    }

    /// Clones this [`Archetype`] instance into a [`Box`].
    #[inline]
    pub fn clone_boxed(&self) -> Box<Self> {
        unsafe { Self::new_boxed(Box::from(&self.0)) }
    }

    /// Returns the underlying list of component types.
    #[inline(always)]
    pub fn ids(&self) -> &[ComponentId] {
        &self.0
    }
}

/// An [`Archetype`] that's stored inline.
///
/// This is what `[T; N]` is to `[T]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InlineArchetype<const N: usize>([ComponentId; N]);

impl<const N: usize> InlineArchetype<N> {
    /// Creates a new [`InlineArchetype`] instance from the provided list of component types.
    ///
    /// # Safety
    ///
    /// The provided [`TypeId`]s must be stored in descending order.
    #[inline(always)]
    pub unsafe fn new(ids: [ComponentId; N]) -> Self {
        Self(ids)
    }
}

impl<const N: usize> AsRef<Archetype> for InlineArchetype<N> {
    #[inline(always)]
    fn as_ref(&self) -> &Archetype {
        // SAFETY:
        //  We know that the provided type IDs are stored in descending order.
        unsafe { Archetype::new_ref(&self.0) }
    }
}

impl<const N: usize> From<InlineArchetype<N>> for Box<Archetype> {
    #[inline(always)]
    fn from(archetype: InlineArchetype<N>) -> Self {
        archetype.as_ref().clone_boxed()
    }
}
