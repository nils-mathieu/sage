use core::alloc::Layout;
use core::any::TypeId;

/// A component that can be attached to an entity.
pub trait Component: 'static {}

macro_rules! impl_Component {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Component for $ty {}
        )*
    };
}

#[rustfmt::skip]
impl_Component!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64,
    &'static str, alloc::string::String
);

impl<T: 'static> Component for alloc::vec::Vec<T> {}
impl<T: 'static + ?Sized> Component for alloc::boxed::Box<T> {}

/// The unique ID of a component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(TypeId);

impl ComponentId {
    /// Returns the [`ComponentId`] instance associated with the provided type.
    #[inline]
    pub fn of<T: Component>() -> Self {
        Self(TypeId::of::<T>())
    }
}

/// Stores meta information about a component.
#[derive(Debug, Clone, Copy)]
pub struct ComponentMeta {
    /// The [`ComponentId`] of the component.
    id: ComponentId,
    /// The memory layout of the component.
    ///
    /// The size stored in this layout must be a multiple of its alignment.
    layout: Layout,
    /// A function that drops the component.
    ///
    /// # Safety
    ///
    /// This function may only be called on a properly initialized instance of the component,
    /// and after it has returned, the component may no longer be used in any way.
    drop_fn: unsafe fn(*mut u8),
}

impl ComponentMeta {
    /// Returns the [`ComponentMeta`] instance associated with the provided type.
    #[inline]
    pub fn of<T: Component>() -> Self {
        Self {
            id: ComponentId::of::<T>(),
            layout: Layout::new::<T>(),
            drop_fn: |ptr| unsafe { ptr.cast::<T>().drop_in_place() },
        }
    }

    /// Returns the [`ComponentId`] of the component.
    #[inline(always)]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    /// Returns the memory layout of the component.
    #[inline(always)]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Returns the alignment requirement of the component.
    #[inline(always)]
    pub fn align(&self) -> usize {
        self.layout.align()
    }

    /// Returns the size of the component.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.layout.size()
    }

    /// Returns the drop function for the component.
    ///
    /// # Safety
    ///
    /// This function may only be called on a properly initialized instance of the component,
    /// and after it has returned, the component may no longer be used in any way.
    #[inline(always)]
    pub unsafe fn drop_in_place(&self, ptr: *mut u8) {
        (self.drop_fn)(ptr)
    }
}
