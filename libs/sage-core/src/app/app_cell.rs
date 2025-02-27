use {
    super::{App, Global},
    crate::{OpaquePtr, Uuid},
    std::marker::PhantomData,
};

/// A wrapper around an [`App`] that makes all accesses unsafe, but no longer require
/// to follow the regular XOR borrow-checker pattern.
///
/// Users are responsible for ensuring the [`App`] is not used in a way that would break
/// Rust's aliasing rules.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct AppCell<'a>(*mut App, PhantomData<&'a App>);

unsafe impl Send for AppCell<'_> {}
unsafe impl Sync for AppCell<'_> {}

impl<'a> AppCell<'a> {
    /// Creates a new [`AppCell`] from an [`App`].
    #[inline]
    pub fn new(app: &'a mut App) -> Self {
        Self(app, PhantomData)
    }

    /// Creates a new read-only [`AppCell`] from a shared [`App`].
    ///
    /// # Safety
    ///
    /// The caller must not access any of the resources of the [`App`] mutably.
    #[inline]
    pub unsafe fn new_read_only(app: &'a App) -> Self {
        Self(app as *const App as *mut App, PhantomData)
    }

    /// Returns a shared reference to the contained [`App`].
    ///
    /// # Safety
    ///
    /// The caller must not access resources of the [`App`] in a way that would break Rust's
    /// aliasing rules.
    #[inline]
    pub unsafe fn get_ref(self) -> &'a App {
        unsafe { &*self.0 }
    }

    /// Returns a mutable reference to the contained [`App`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the whole [`App`] is not accessed in any other ways while
    /// the returning reference is alive.
    pub unsafe fn get_mut(self) -> &'a mut App {
        unsafe { &mut *self.0 }
    }

    /// Gets the pointer to one of the global resources of the application.
    #[inline]
    pub fn global_raw(self, uuid: Uuid) -> Option<OpaquePtr> {
        unsafe { self.get_ref().globals().get_raw(uuid).map(|x| x.data()) }
    }

    /// Gets a reference to one of the global resources of the application.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the global resource is not accessed mutable while the
    /// returned reference is alive.
    #[inline]
    pub unsafe fn global<T: Global>(self) -> Option<&'a T> {
        unsafe { self.global_raw(T::UUID).map(|x| x.as_ref()) }
    }

    /// Gets a mutable reference to one of the global resources of the application.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the global resource is not accessed in any way
    /// while the returned reference is alive.
    #[inline]
    pub unsafe fn global_mut<T: Global>(self) -> Option<&'a mut T> {
        unsafe { self.global_raw(T::UUID).map(|x| x.as_mut()) }
    }
}
