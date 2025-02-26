use {
    crate::{TypeUuid, Uuid, opaque_ptr::OpaquePtr},
    std::{alloc::Layout, borrow::Borrow},
};

/// Stores information about the memory layout of a component, as well as how to clean it up.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ComponentInfo {
    /// The UUID of the component.
    pub uuid: Uuid,
    /// A debug name for the component.
    pub debug_name: &'static str,
    /// The memory layout of the component.
    ///
    /// The size part of the layout must be aligned to the component's alignment.
    pub layout: Layout,
    /// A function that must be called on the component in order to release the resources it may
    /// hold. `None` if the component does not require any cleanup.
    pub drop_fn: Option<unsafe extern "C" fn(data: OpaquePtr)>,
}

impl ComponentInfo {
    /// Creates a new [`ComponentLayout`] instance for the provided Rust type.
    pub fn of<T: Component>() -> &'static Self {
        unsafe extern "C" fn drop_fn<T: Component>(data: OpaquePtr) {
            unsafe { std::ptr::drop_in_place(data.as_ptr::<T>()) }
        }

        trait ProvideInfo {
            const INFO: ComponentInfo;
        }

        impl<T: Component> ProvideInfo for T {
            const INFO: ComponentInfo = ComponentInfo {
                uuid: T::UUID,
                debug_name: T::DEBUG_NAME,
                layout: Layout::new::<T>().pad_to_align(),
                drop_fn: if std::mem::needs_drop::<T>() {
                    Some(drop_fn::<T>)
                } else {
                    None
                },
            };
        }

        &T::INFO
    }
}

impl PartialEq for ComponentInfo {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for ComponentInfo {}

impl std::hash::Hash for ComponentInfo {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

impl std::fmt::Debug for ComponentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentInfo")
            .field("uuid", &self.uuid)
            .field("debug_name", &self.debug_name)
            .field("layout", &self.layout)
            .finish()
    }
}

impl Borrow<Uuid> for ComponentInfo {
    #[inline(always)]
    fn borrow(&self) -> &Uuid {
        &self.uuid
    }
}

impl Borrow<Uuid> for &'_ ComponentInfo {
    #[inline(always)]
    fn borrow(&self) -> &Uuid {
        &self.uuid
    }
}

/// A trait that describes component in the Rust type system.
pub trait Component: 'static + Send + Sync + TypeUuid {
    /// The debug name of the component.
    const DEBUG_NAME: &'static str = std::any::type_name::<Self>();
}

/// A registry responsible for storing information about available components.
#[derive(Default)]
pub struct ComponentRegistry(
    hashbrown::HashSet<&'static ComponentInfo, foldhash::fast::FixedState>,
);

impl ComponentRegistry {
    /// Registers a component with the registry.
    pub fn register<T: Component>(&mut self) -> &'static ComponentInfo {
        let info = ComponentInfo::of::<T>();
        unsafe { self.register_raw(info) }
        info
    }

    /// Registers a component with the registry without using the Rust type system.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the inserted component's UUID is controlled by them, meaning
    /// that nobody else will attempt to insert a component with the same UUID but different
    /// properties.
    #[inline]
    pub unsafe fn register_raw(&mut self, info: &'static ComponentInfo) {
        self.0.insert(info);
    }

    /// Gets information about a particular component based on its UUID.
    #[inline(always)]
    pub fn get_by_uuid(&self, uuid: Uuid) -> Option<&'static ComponentInfo> {
        self.0.get(&uuid).copied()
    }
}
