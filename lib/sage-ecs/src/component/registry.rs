use alloc::{boxed::Box, vec::Vec};
use core::alloc::Layout;
#[cfg(feature = "rust-components")]
use core::any::TypeId;

#[cfg(feature = "rust-components")]
use super::{Bundle, Component};
#[cfg(feature = "rust-components")]
use crate::utility::{NoopBuildHasher, NoopHashMap};

/// A function that is responsible for dropping a component instance.
///
/// For regular Rust types, this function is generally just `std::ptr::drop_in_place`. But external
/// components may require a custom drop function.
///
/// # Safety
///
/// After this function has been called on a memory location owning a component instance, the
/// memory location must be considered *uninitialized* and may not be accessed again.
pub type DropFn = unsafe fn(*mut u8);

/// Stores information about a component type.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// The name of the component type. This is mainly used for debugging purposes.
    pub name: Box<str>,
    /// A function that must be called in order to drop a component instance.
    ///
    /// This function is responsible for freeing any resources that the component instance may
    /// own.
    ///
    /// If the component does not need to be dropped, this field is `None`.
    pub drop_fn: Option<DropFn>,
    /// The memory layout of the component type. A continuous block of bytes that fits such layout
    /// is suitable for storing an instance of this component.
    pub layout: Layout,
}

impl ComponentInfo {
    /// Returns the [`ComponentInfo`] associated with the provided Rust type.
    pub fn of<T: 'static>() -> Self {
        Self {
            name: core::any::type_name::<T>().into(),
            drop_fn: if core::mem::needs_drop::<T>() {
                Some(|p| unsafe { core::ptr::drop_in_place(p as *mut T) })
            } else {
                None
            },
            layout: Layout::new::<T>(),
        }
    }
}

/// Represents the ID of a component type.
pub type ComponentId = usize;

/// Stores information about a component bundle.
#[derive(Debug, Clone)]
pub struct BundleInfo {
    /// The name of the component bundle. This is mainly used for debugging purposes.
    pub name: Box<str>,
    /// The components that make up this bundle.
    pub components: Box<[ComponentId]>,
}

/// Represents the ID of a bundle type.
pub type BundleId = usize;

/// Stores information about registered component and component bundles.
pub struct Registry {
    /// The components that have been registered so far. `ComponentId`s are used to index into this
    /// vector.
    components: Vec<ComponentInfo>,
    /// The bundles that have been registered so far. `BundleId`s are used to index into this
    /// vector.
    bundles: Vec<BundleInfo>,

    /// Maps Rust types to their associated component IDs.
    #[cfg(feature = "rust-components")]
    rust_components: NoopHashMap<TypeId, ComponentId>,
    /// Maps Rust types to their associated bundle IDs.
    #[cfg(feature = "rust-components")]
    rust_bundles: NoopHashMap<TypeId, BundleId>,
}

impl Registry {
    /// Creates a new [`Registry`] instance with no registered components or bundles.
    pub const fn new() -> Self {
        Self {
            components: Vec::new(),
            bundles: Vec::new(),
            #[cfg(feature = "rust-components")]
            rust_components: NoopHashMap::with_hasher(NoopBuildHasher),
            #[cfg(feature = "rust-components")]
            rust_bundles: NoopHashMap::with_hasher(NoopBuildHasher),
        }
    }

    /// Registers a Rust component.
    ///
    /// If the component has already been previously registered, this function will return
    /// the existing component ID.
    #[cfg(feature = "rust-components")]
    pub fn register_rust_component<T: Component>(&mut self) -> ComponentId {
        *self
            .rust_components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| {
                let info = ComponentInfo::of::<T>();

                // Can't use `register_component` here because that borrows the whole register
                // mutably.
                let id = self.components.len();
                self.components.push(info);
                id
            })
    }

    /// Registers a new component bundle.
    ///
    /// # Remarks
    ///
    /// If this function is called twice with the same [`ComponentInfo`] instance, two separate
    /// components will be registered.
    pub fn register_component(&mut self, info: ComponentInfo) -> ComponentId {
        let id = self.components.len();
        self.components.push(info);
        id
    }

    /// Returns a slice of all registered components.
    ///
    /// This slice can be indexed by [`ComponentId`]s to retrieve the associated [`ComponentInfo`].
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn components(&self) -> &[ComponentInfo] {
        &self.components
    }

    /// Registers a static Rust bundle.
    ///
    /// If the bundle has already been registered, this function will return the existing bundle
    /// ID.
    #[cfg(feature = "rust-components")]
    pub fn register_rust_bundle<B: Bundle>(&mut self) -> BundleId {
        // FIXME: We can probably avoid.

        let type_id = TypeId::of::<B>();
        if self.rust_bundles.contains_key(&type_id) {
            unsafe { *self.rust_bundles.get(&type_id).unwrap_unchecked() }
        } else {
            let components = B::register_components(self);
            let id = self.register_bundle(BundleInfo {
                name: core::any::type_name::<B>().into(),
                components,
            });
            self.rust_bundles.insert_unique_unchecked(type_id, id);
            id
        }
    }

    /// Registers a new component bundle.
    ///
    /// # Remarks
    ///
    /// If this function is called twice with the same [`BundleInfo`] instance, two separate bundles
    /// will be registered.
    pub fn register_bundle(&mut self, info: BundleInfo) -> BundleId {
        let id = self.bundles.len();
        self.bundles.push(info);
        id
    }

    /// Returns a slice of all registered bundles.
    ///
    /// This slice can be indexed by [`BundleId`]s to retrieve the associated [`BundleInfo`].
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn bundles(&self) -> &[BundleInfo] {
        &self.bundles
    }
}

impl Default for Registry {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}
