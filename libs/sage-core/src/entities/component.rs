use {
    crate::{TypeUuid, opaque_ptr::OpaquePtr},
    std::alloc::Layout,
};

/// Stores information about the memory layout of a component, as well as how to clean it up.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ComponentLayout {
    /// The memory layout of the component.
    pub memory: Layout,
    /// A function that must be called on the component in order to release the resources it may
    /// hold. `None` if the component does not require any cleanup.
    pub drop_fn: Option<unsafe extern "C" fn(data: OpaquePtr)>,
}

impl ComponentLayout {
    /// Creates a new [`ComponentLayout`] instance for the provided Rust type.
    pub fn of<T: Component>() -> Self {
        unsafe extern "C" fn drop_fn<T: Component>(data: OpaquePtr) {
            unsafe { std::ptr::drop_in_place(data.as_ptr::<T>()) }
        }

        Self {
            memory: Layout::new::<T>(),
            drop_fn: if std::mem::needs_drop::<T>() {
                Some(drop_fn::<T>)
            } else {
                None
            },
        }
    }
}

/// Stores information about a potential entity component.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComponentInfo {
    /// A debug name for the component.
    debug_name: &'static str,
    /// The [`ComponentLayout`] of the component.
    layout: ComponentLayout,
}

/// A trait that describes component in the Rust type system.
pub trait Component: 'static + Send + Sync + TypeUuid {
    /// The debug name of the component.
    const DEBUG_NAME: &'static str = std::any::type_name::<Self>();
}
