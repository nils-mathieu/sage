use core::alloc::Layout;
use core::hash::{BuildHasherDefault, Hasher};
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

use alloc::boxed::Box;
use hashbrown::HashMap;

use super::archetype::{Archetype, InlineArchetype};
use super::component::{Component, ComponentId, ComponentMeta};

/// A hasher that does not hash anything.
///
/// This is useful because the standard [`TypeId`] type is already properly hashed.
#[derive(Default)]
struct NoopHasher(u64);

impl Hasher for NoopHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline(never)]
    #[cold]
    fn write(&mut self, _bytes: &[u8]) {
        panic!("the NoopHasher should not be used to hash anything other than `u64`");
    }

    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}

/// A component that's part of an [`EntityLayout`]
#[derive(Debug, Clone, Copy)]
pub struct Field {
    /// The offset of the component within its owning entity.
    pub offset: usize,
    /// The memory layout of the component.
    pub meta: ComponentMeta,
}

/// The type that's used to store the component offsets within an entity.
type ComponentMap = HashMap<ComponentId, Field, BuildHasherDefault<NoopHasher>>;

/// Stores the memory layout of an entity with a specific archetype.
pub struct EntityLayout {
    /// The memory layout of the entity.
    layout: Layout,
    /// A map that associates the type of each component with its offset within the entity
    /// structure.
    components: ComponentMap,
}

impl EntityLayout {
    /// Creates a new [`EntityLayout`] from the provided list of components.
    ///
    /// # Safety
    ///
    /// The provided iterator must return the components in the decending alignment order.
    ///
    /// The components must be unique.
    pub unsafe fn new_unchecked<I>(iter: I) -> Self
    where
        I: Iterator<Item = ComponentMeta>,
    {
        let (lower, _) = iter.size_hint();
        let mut components = ComponentMap::with_capacity_and_hasher(lower, Default::default());

        // This function basically constructs a Rust structure by hand, using the provided
        // components as fields. Because the provided iterator returns a decending alignment
        // order, we know that the struct we're constructing will:
        // 1. be properly aligned,
        // 2. contain the least amount of padding possible.

        let mut offset = 0;
        let mut align = 0;

        for meta in iter {
            // We know that the offset is already aligned to the alignment of the previous
            // component because alignment is always decending.
            debug_assert_eq!(offset % meta.layout().align(), 0);

            // Because of the decending order, the first component we encounter will be the
            // one with the highest alignment. It's therefor the alignment of the whole
            // structure.
            if align == 0 {
                align = meta.layout().align();
            }

            components.insert_unique_unchecked(meta.id(), Field { offset, meta });

            offset += meta.layout().pad_to_align().size();
        }

        if align == 0 {
            // The structure is empty, so we can use the minimum alignment.
            align = 1;
        }

        // The `pad_to_align` call is necessary because it's possible that the total size of
        // the fields do not add up to a multiple of the alignment.
        let layout = Layout::from_size_align(offset, align)
            .expect("failed to build a layout for the archetype")
            .pad_to_align();

        Self { layout, components }
    }

    /// Returns the size of an entity with this layout.
    ///
    /// This is always a multiple of the archetype's alignment.
    #[inline(always)]
    pub const fn size(&self) -> usize {
        self.layout.size()
    }

    /// Returns the alignment of an entity with this layout.
    ///
    /// All entities with this layout must be aligned to this value.
    ///
    /// The value returned by this function is guranateed to be a power of two.
    #[inline(always)]
    pub const fn align(&self) -> usize {
        self.layout.align()
    }

    /// Returns a [`Layout`] that can be used to store an array of entities with this layout.
    ///
    /// If the layout overflows the size of an `usize`, this function returns [`None`].
    pub fn layout_for_array(&self, n: usize) -> Option<Layout> {
        let size = self.size().checked_mul(n)?;
        let align = self.align();

        // SAFETY:
        //  We know that the size is a multiple of the alignment because the size of the
        //  entity is a multiple of the alignment.
        Some(unsafe { Layout::from_size_align_unchecked(size, align) })
    }

    /// Returns a [`NonNull<()>`] pointer aligned to the alignment of this layout.
    #[inline(always)]
    pub const fn dangling(&self) -> NonNull<u8> {
        #[cfg(miri)]
        let ptr = core::ptr::invalid_mut(self.align());
        #[cfg(not(miri))]
        let ptr = core::ptr::null_mut();

        // SAFETY:
        //  We know that `align()` returns a power of two, which cannot be zero.
        unsafe { NonNull::new_unchecked(ptr) }
    }

    /// Returns the number of components in this archetype.
    #[inline]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns an iterator over the [`Field`]s of the archetype.
    #[inline]
    pub fn components(&self) -> impl '_ + Iterator<Item = &Field> {
        self.components.values()
    }

    /// Returns the field associated with the provided type ID, if it exists.
    #[inline]
    pub fn field_of(&self, id: ComponentId) -> Option<&Field> {
        self.components.get(&id)
    }

    /// Returns whether the archetype contains a component of the provided type.
    #[inline]
    pub fn has_component(&self, id: ComponentId) -> bool {
        self.components.contains_key(&id)
    }
}

/// Types that have an associated archetype.
///
/// # Safety
///
/// - The associated [`ArchetypeStore`] type must have a coherent implementation of
///   [`AsRef<Archetype>`] and [`Into<Box<Archetype>>`]. Specifically, both of those implementations
///   must return the same archetype.
///
/// - The archetype that is returned by [`archetype`] must be coherent with the [`EntityLayout`]
///   returned by [`layout`].
///
/// [`archetype`]: Self::archetype
/// [`layout`]: Self::layout
pub unsafe trait IntoEntityLayout {
    /// The type that's responsible for storing an instance of [`Archetype`]. This is usually
    /// a `Box<Archetype>`, a `&'static Archetype` or a [`InlineArchetype`].
    type ArchetypeStore<'a>: AsRef<Archetype> + Into<Box<Archetype>>
    where
        Self: 'a;

    /// Returns the archetype associated with this collection of components.
    fn archetype(&self) -> Self::ArchetypeStore<'_>;

    /// Returns the [`EntityLayout`] associated with the [`Archetype`] returned by
    /// [`archetype`].
    ///
    /// [`archetype`]: Self::archetype
    fn into_layout(self) -> EntityLayout;
}

/// A trait that can be used to initialize an entity.
///
/// # Implicit Archetype
///
/// Implementors of this trait are implicitly associated with a certain entity archetype, and are
/// only able to initialize entities with that archetype.
///
/// # Safety
///
/// The pointers passed to the `write_component` function must be valid and the ownership of the
/// pointed values is logically transferred to the function when it called.
///
/// Additionally, the `TypeId` that's passed along with the pointer must the same one as the
/// component that's being transferred.
///
/// If the implementation implements [`Send`] or [`Sync`], then all of the components that it
/// initializes must also implement [`Send`] or [`Sync`].
///
/// [`initialize_entity`]: Self::initialize_entity
pub unsafe trait InitializeEntity {
    /// Initializes the entity stored in the provided memory location.
    ///
    /// # Safety
    ///
    /// The provided
    unsafe fn write_components<F>(self, write_component: F)
    where
        F: FnMut(ComponentId, *mut u8);
}

unsafe impl<T: Component> InitializeEntity for T {
    #[inline(always)]
    unsafe fn write_components<F>(self, mut write_component: F)
    where
        F: FnMut(ComponentId, *mut u8),
    {
        let mut val = ManuallyDrop::new(self);
        write_component(ComponentId::of::<T>(), &mut val as *mut _ as *mut _);
    }
}

/// An (eventually dynamic) collection of components.
///
/// # Safety
///
/// - The archetype described by the [`IntoEntityLayout`] implementation of this type must be
///   cohrent with its [`InitializeEntity`] implementation.
pub unsafe trait Components: InitializeEntity {
    /// The dynamic [`Archetype`] associated with this collection of components.
    type Archetype<'a>: IntoEntityLayout
    where
        Self: 'a;

    /// Returns the [`IntoEntityLayout`] implementation that's associated with this collection
    /// of components.
    fn archetype(&self) -> Self::Archetype<'_>;
}

unsafe impl<T: StaticComponents> Components for T {
    type Archetype<'a> = T::Archetype
    where
        Self: 'a;

    #[inline(always)]
    fn archetype(&self) -> Self::Archetype<'_> {
        <Self as StaticComponents>::archetype()
    }
}

/// The [`IntoEntityLayout`] implementation for single components.
pub struct SingleComponentArchetype<T>(PhantomData<T>);

unsafe impl<T: Component> IntoEntityLayout for SingleComponentArchetype<T> {
    type ArchetypeStore<'a> = InlineArchetype<1>;

    #[inline(always)]
    fn archetype(&self) -> Self::ArchetypeStore<'_> {
        unsafe { InlineArchetype::new([ComponentId::of::<T>()]) }
    }

    #[inline(always)]
    fn into_layout(self) -> EntityLayout {
        unsafe { EntityLayout::new_unchecked(core::iter::once(ComponentMeta::of::<T>())) }
    }
}

unsafe impl<T: Component> StaticComponents for T {
    type Archetype = SingleComponentArchetype<T>;

    #[inline(always)]
    fn archetype() -> Self::Archetype {
        SingleComponentArchetype(PhantomData)
    }
}

/// Like [`Components`], but the [`IntoEntityLayout`] implementation is known statically.
///
/// # Safety
///
/// - The [`IntoEntityLayout`] implementation of [`Archetype`] must be coherent with the
///   [`InitializeEntity`] implementation of this type.
///
/// [`Archetype`]: Self::Archetype
pub unsafe trait StaticComponents: InitializeEntity {
    /// The static [`Archetype`] associated with this collection of components.
    type Archetype: IntoEntityLayout;

    /// Returns the [`IntoEntityLayout`] implementation that's associated with this collection
    /// of components.
    fn archetype() -> Self::Archetype;
}

/// The [`IntoEntityLayout`] implementation for tuples.
pub struct TupleArchetypes<C>(PhantomData<C>);

macro_rules! impl_for_tuple {
    (has_duplicates $first:expr, $($ty:expr,)*) => {
        $( $first == $ty || )* impl_for_tuple!(has_duplicates $($ty,)*)
    };

    (has_duplicates) => {
        false
    };

    (count) => { 0 };

    (count $($ty:ident,)*) => {
        [ $( impl_for_tuple!(count_ $ty), )* ].len()
    };

    (count_ $ty:ident) => { () };

    ($($ty:ident),*) => {
        const _: () = {
            unsafe impl<$($ty: Component),*> InitializeEntity for ($($ty,)*) {
                #[inline(always)]
                #[allow(unused_mut, unused_variables, non_snake_case)]
                unsafe fn write_components<Func>(mut self, mut write_component: Func)
                where
                Func: FnMut(ComponentId, *mut u8),
                {
                    let ($($ty,)*) = self;

                    $(
                        {
                            let mut val = ManuallyDrop::new($ty);
                            write_component(ComponentId::of::<$ty>(), &mut val as *mut _ as *mut _);
                        }
                    )*
                }
            }

            unsafe impl<$($ty: Component),*> IntoEntityLayout for TupleArchetypes<($($ty,)*)> {
                type ArchetypeStore<'a> = InlineArchetype<{ impl_for_tuple!(count $($ty,)*) }>
                where
                    Self: 'a;

                #[inline(always)]
                fn archetype(&self) -> Self::ArchetypeStore<'_> {
                    if impl_for_tuple!(has_duplicates $( ComponentId::of::<$ty>() ,)* ) {
                        panic!("found duplicate component type in tuple");
                    }

                    let mut array = [$(ComponentMeta::of::<$ty>(),)*];

                    array.sort_unstable_by_key(|meta: &ComponentMeta| meta.align());

                    unsafe { InlineArchetype::new([$(ComponentId::of::<$ty>()),*]) }
                }

                fn into_layout(self) -> EntityLayout {
                    if impl_for_tuple!(has_duplicates $( ComponentId::of::<$ty>() ,)* ) {
                        panic!("found duplicate component type in tuple");
                    }

                    let mut array = [$(ComponentMeta::of::<$ty>(),)*];

                    array.sort_unstable_by(|a: &ComponentMeta, b: &ComponentMeta| b.align().cmp(&a.align()));

                    unsafe { EntityLayout::new_unchecked(array.iter().copied()) }
                }
            }

            unsafe impl<$($ty: Component),*> StaticComponents for ($($ty,)*) {
                type Archetype = TupleArchetypes<($($ty,)*)>;

                #[inline(always)]
                fn archetype() -> Self::Archetype {
                    TupleArchetypes(PhantomData)
                }
            }
        };
    };
}

impl_for_tuple!();
impl_for_tuple!(A);
impl_for_tuple!(A, B);
impl_for_tuple!(A, B, C);
impl_for_tuple!(A, B, C, D);
impl_for_tuple!(A, B, C, D, E);
impl_for_tuple!(A, B, C, D, E, F);
