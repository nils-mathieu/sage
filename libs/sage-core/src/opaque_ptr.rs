use std::{alloc::Layout, num::NonZero, ptr::NonNull};

/// A pointer to some opaque data.
///
/// Unlike a regular raw pointer, this type implements `Send` and `Sync` inconditionally. It is
/// the responsibility of the user to ensure that the referenced data is actually safe to share
/// across threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OpaquePtr(NonNull<()>);

impl OpaquePtr {
    /// Creates a new [`OpaquePtr<P>`] instance from the provided non-null pointer.
    #[inline]
    pub const fn from_non_null<P>(p: NonNull<P>) -> Self {
        Self(p.cast())
    }

    /// Creates a new [`OpaquePtr<P>`] instance from the provided raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provide pointer is non-null.
    #[inline]
    pub const unsafe fn from_raw<P>(p: *mut P) -> Self {
        unsafe { Self::from_non_null(NonNull::new_unchecked(p)) }
    }

    /// Creates a new [`OpaquePtr<P>`] instance from the provided reference.
    #[inline]
    pub const fn from_ref<P>(p: &P) -> Self {
        unsafe { Self::from_raw(p as *const P as *mut P) }
    }

    /// Creates a new [`OpaquePtr<P>`] instance from the provided mutable reference.
    #[inline]
    pub const fn from_mut<P>(p: &mut P) -> Self {
        unsafe { Self::from_raw(p as *mut P) }
    }

    /// Creates a new [`OpaquePtr<P>`] instance from the provided non-zero address.
    #[inline]
    pub const fn without_provenance(addr: NonZero<usize>) -> Self {
        Self(NonNull::without_provenance(addr))
    }

    /// Creates a new [`OpaquePtr<P>`] instance that points to a dangling address but is suitably
    /// aligned for the provided layout.
    #[inline]
    pub const fn dangling_for(layout: Layout) -> Self {
        // SAFETY: An alignment is always non-zero.
        let addr = unsafe { NonZero::new_unchecked(layout.align()) };

        Self::without_provenance(addr)
    }

    /// Offsets the pointer by `offset` bytes and returns the result.
    #[inline(always)]
    pub fn byte_add(self, offset: usize) -> Self {
        unsafe { Self(NonNull::new_unchecked(self.0.as_ptr().byte_add(offset))) }
    }

    /// Returns a reference to the data pointed to by this pointer.
    #[inline(always)]
    pub const fn as_ptr<P>(self) -> *mut P {
        self.0.as_ptr().cast()
    }

    /// Returns a reference to the data pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the stored address is valid for the lifetime of the returned
    /// reference.
    #[inline(always)]
    pub const unsafe fn as_ref<'a, P>(self) -> &'a P {
        unsafe { &*self.as_ptr::<P>() }
    }

    /// Returns a mutable reference to the data pointed to by this pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the stored address is valid for the lifetime of the returned
    /// reference.
    #[inline(always)]
    pub const unsafe fn as_mut<'a, P>(self) -> &'a mut P {
        unsafe { &mut *self.as_ptr::<P>() }
    }
}

unsafe impl Send for OpaquePtr {}
unsafe impl Sync for OpaquePtr {}
