use {
    crate::{TypeUuid, Uuid, opaque_ptr::OpaquePtr},
    std::ops::{Index, IndexMut},
};

/// A raw global stored in a [`Globals`] collection.
///
/// In Rust, this can be thought of as a `Box<dyn Global>`.
#[repr(C)]
pub struct RawGlobal {
    /// The data itself.
    data: OpaquePtr,

    /// A debug name for the global resource.
    ///
    /// Used exclusively for debugging purposes.
    debug_name: &'static str,

    /// The function responsible for cleaning up the global resource once it is no longer needed.
    ///
    /// # Safety
    ///
    /// Once this function has been called, the referenced global data must not be used anymore.
    drop_fn: unsafe extern "C" fn(OpaquePtr),
}

impl RawGlobal {
    /// Creates a new [`RawGlobal`] instance from the provided value. It must implement the
    /// [`Global`] trait.
    pub fn new<G: Global>(data: Box<G>) -> Self {
        unsafe extern "C" fn drop_fn<G: Global>(data: OpaquePtr) {
            _ = unsafe { Box::from_raw(data.as_ptr::<G>()) };
        }

        Self {
            // SAFETY: A boxed value is always non-null.
            data: unsafe { OpaquePtr::from_raw(Box::into_raw(data)) },
            debug_name: G::DEBUG_NAME,

            drop_fn: drop_fn::<G>,
        }
    }

    /// Returns the debug name of the global resource.
    ///
    /// This value is used exclusively for debugging purposes.
    #[inline(always)]
    pub fn debug_name(&self) -> &'static str {
        self.debug_name
    }

    /// Returns the opaque pointer to the global resource.
    #[inline(always)]
    pub fn data(&self) -> OpaquePtr {
        self.data
    }
}

impl Drop for RawGlobal {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.drop_fn)(self.data) };
    }
}

impl std::fmt::Debug for RawGlobal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawGlobal {{ debug_name: {:?} }}", self.debug_name)
    }
}

/// Contains a collection of global resources that can be used by all parts of the engine.
///
/// # What are globals?
///
/// A **global** value is, as the name suggests, a value that is accessible from anywhere in the
/// program. The [`App`] type keeps a reference to a [`Globals`] collection, which itself stores
/// references to all registered global resources for the application.
///
/// [`App`]: crate::App
#[derive(Default)]
pub struct Globals(hashbrown::HashMap<Uuid, RawGlobal, foldhash::fast::FixedState>);

impl Globals {
    /// Registers a new global resource into the collection.
    ///
    /// # Panics
    ///
    /// This function panics if a global resource with the same UUID has already been registered
    /// previously to the collection.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided UUID corresponds to the actual type referenced by
    /// the [`RawGlobal`] instance.
    #[track_caller]
    pub unsafe fn register_raw(&mut self, uuid: Uuid, value: RawGlobal) {
        assert!(
            self.0.try_insert(uuid, value).is_ok(),
            "A global resource with UUID {uuid:?} has already been registered",
        );
    }

    /// Ensures that a global resource is registered into the collection with the given UUID.
    ///
    /// If the resource is already registered, this function does nothing. Otherwise, it calls
    /// the provided closure to create the resource and registers it.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided closure returns a valid [`RawGlobal`] instance,
    /// which itself is correctly associated with the provided UUID.
    pub unsafe fn register_raw_with(&mut self, uuid: Uuid, f: impl FnOnce() -> RawGlobal) {
        self.0.entry(uuid).or_insert_with(f);
    }

    /// Registers a new global resource into the collection.
    ///
    /// # Panics
    ///
    /// This function panics if the global resource was already registered previously (or if one
    /// with the same UUID was, at least).
    #[track_caller]
    pub fn register<G: Global>(&mut self, value: Box<G>) {
        unsafe { self.register_raw(G::UUID, RawGlobal::new(value)) };
    }

    /// Ensures that a global resource is registered into the collection with the given type.
    ///
    /// If the resource is already registered, this function does nothing. Otherwise, it calls
    /// the provided closure to create the resource and registers it.
    pub fn register_with<G: Global>(&mut self, f: impl FnOnce() -> Box<G>) {
        unsafe { self.register_raw_with(G::UUID, || RawGlobal::new(f())) };
    }

    /// Retrieves a global resource from the collection by its [`Uuid`].
    ///
    /// # Returns
    ///
    /// If a global resource with the provided ID exists, this function returns a reference to it.
    ///
    /// Otherwise, this function returns [`None`].
    #[inline]
    pub fn get_raw(&self, uuid: Uuid) -> Option<&RawGlobal> {
        self.0.get(&uuid)
    }

    /// Retrieves a mutable global resource from the collection by its [`Uuid`].
    ///
    /// # Returns
    ///
    /// If a global resource with the provided ID exists, this function returns a mutable reference
    /// to it.
    ///
    /// Otherwise, this function returns [`None`].
    #[inline]
    pub fn get_raw_mut(&mut self, uuid: Uuid) -> Option<&mut RawGlobal> {
        self.0.get_mut(&uuid)
    }

    /// Gets the global resource associated with the provided [`Uuid`].
    ///
    /// # Returns
    ///
    /// If a global resource of type `G` has been registered previously, this function returns
    /// a reference to it.
    pub fn try_get<G: Global>(&self) -> Option<&G> {
        self.get_raw(G::UUID)
            .map(|raw| unsafe { raw.data.as_ref::<G>() })
    }

    /// Gets the global resource associated with the provided [`Uuid`].
    ///
    /// # Panics
    ///
    /// This function panics if no global resource of type `G` has been registered previously.
    #[track_caller]
    pub fn get<G: Global>(&self) -> &G {
        self.try_get::<G>()
            .unwrap_or_else(|| missing_global(G::DEBUG_NAME))
    }

    /// Gets the global resource associated with the provided [`Uuid`].
    ///
    /// # Returns
    ///
    /// If a global resource of type `G` has been registered previously, this function returns
    /// a mutable reference to it.
    pub fn try_get_mut<G: Global>(&mut self) -> Option<&mut G> {
        self.get_raw_mut(G::UUID)
            .map(|raw| unsafe { raw.data.as_mut::<G>() })
    }

    /// Gets the global resource associated with the provided [`Uuid`].
    ///
    /// # Panics
    ///
    /// This function panics if no global resource of type `G` has been registered previously.
    #[track_caller]
    pub fn get_mut<G: Global>(&mut self) -> &mut G {
        self.try_get_mut::<G>()
            .unwrap_or_else(|| missing_global(G::DEBUG_NAME))
    }
}

impl Index<Uuid> for Globals {
    type Output = RawGlobal;

    #[track_caller]
    fn index(&self, uuid: Uuid) -> &RawGlobal {
        self.get_raw(uuid).unwrap_or_else(|| unknown_uuid(uuid))
    }
}

impl IndexMut<Uuid> for Globals {
    #[track_caller]
    fn index_mut(&mut self, uuid: Uuid) -> &mut RawGlobal {
        self.get_raw_mut(uuid).unwrap_or_else(|| unknown_uuid(uuid))
    }
}

/// A function that panics when a global resource is not found given its UUID.
#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn unknown_uuid(uuid: Uuid) -> ! {
    panic!("Found no global resource with UUID {uuid}");
}

/// A function that panics when a global resource is not found given its name.
#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn missing_global(name: &'static str) -> ! {
    panic!("Missing global resource: {name:?}");
}

/// A trait to represent a global resource. Rust types that implement this trait can be registered
/// easily into a [`Globals`] collection.
pub trait Global: 'static + Send + Sync + TypeUuid {
    /// A debug name for the global resource.
    ///
    /// This is used exclusively for debugging purposes.
    const DEBUG_NAME: &'static str = std::any::type_name::<Self>();
}
