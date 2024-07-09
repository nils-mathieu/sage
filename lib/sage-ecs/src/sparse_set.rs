//! Provides a "sparse set" implementation.

use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::utility::assert_unchecked;

/// A trait for types that can be used as an index into the dense vector of a [`SparseSet`].
///
/// # Safety
///
/// The [`to_usize`] method must always return a value that is less or equal to [`SENTINEL`].
///
/// [`to_usize`]: DenseIndex::to_usize
/// [`SENTINEL`]: DenseIndex::SENTINEL
pub unsafe trait DenseIndex: Copy + __private::Sealed {
    /// The sentinel value for the index type.
    ///
    /// This is usually the maximum value that the index type can represent.
    const SENTINEL: Self;

    /// Constructs a new index from the given `usize` value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `val` is less or equal to [`SENTINEL`] (when converted in a
    /// `usize` value).
    ///
    /// [`SENTINEL`]: DenseIndex::SENTINEL
    unsafe fn from_usize_unchecked(val: usize) -> Self;

    /// Turns the index into a `usize` value.
    fn to_usize(self) -> usize;
}

mod __private {
    pub trait Sealed {}

    impl Sealed for usize {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
}

macro_rules! impl_dense_index {
    ($($ty:ty),*) => {
        $(
            unsafe impl DenseIndex for $ty {
                const SENTINEL: Self = <$ty>::MAX;

                #[inline]
                unsafe fn from_usize_unchecked(val: usize) -> Self {
                    unsafe { assert_unchecked(val <= Self::MAX as usize) };
                    val as Self
                }

                #[inline]
                fn to_usize(self) -> usize {
                    self as usize
                }
            }
        )*
    };
}

impl_dense_index!(usize, u64, u32, u16, u8);

/// A vacant entry in a [`SparseSet`].
pub struct VacantEntry<'a, T, I> {
    /// A reference into the sparse array. The pointed value must be updated when the entry is
    /// populated.
    dense_index: &'a mut I,
    /// The vector that will hold the inserted value.
    ///
    /// There are two invariants that must be maintained:
    ///
    /// 1. The vector must already have reserved space for the new value.
    /// 2. The current length of the vector must be strinctly less than the sentinel
    /// value of the dense index type.
    dense: &'a mut Vec<T>,
}

impl<'a, T, I: DenseIndex> VacantEntry<'a, T, I> {
    /// Inserts a value into the entry.
    pub fn insert(self, value: T) -> &'a mut T {
        let len = self.dense.len();
        // SAFETY: The length of the vector must be strictly bellow the sentinel value.
        *self.dense_index = unsafe { I::from_usize_unchecked(len) };

        // SAFETY: The vector must have reserved space for the new value.
        unsafe {
            let slot = self.dense.as_mut_ptr().add(len);
            slot.write(value);
            self.dense.set_len(len.unchecked_add(1));
            &mut *slot
        }
    }
}

/// An entry in a [`SparseSet`].
pub enum Entry<'a, T, I> {
    /// The entry is occupied by a value.
    Occupied(&'a mut T),
    /// The entry is vacant and can be filled.
    Vacant(VacantEntry<'a, T, I>),
}

impl<'a, T, I: DenseIndex> Entry<'a, T, I> {
    /// Gets the value of the entry or inserts the provided default value if the entry is vacant.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn or_insert(self, value: T) -> &'a mut T {
        match self {
            Entry::Occupied(v) => v,
            Entry::Vacant(entry) => entry.insert(value),
        }
    }

    /// Gets the value of the entry or inserts a new value using the provided closure if the entry
    /// is vacant.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn or_insert_with<F: FnOnce() -> T>(self, f: F) -> &'a mut T {
        match self {
            Entry::Occupied(v) => v,
            Entry::Vacant(entry) => entry.insert(f()),
        }
    }
}

/// A container that allows mapping arbitrary keys of type `usize` to values of type `T` without
/// using hashes or other complex data structures.
///
/// The main idea is to use the `usize` key as an index into a "sparse" vector, which contains the
/// actual values. This allows for very fast lookups and insertions, but has the downside of
/// consuming more memory because of the "holes" that may appear in the vector.
///
/// To mitigate this issue, the `SparseSet` type can change the internal dense index used to
/// access the dense vector, treading the maximum number of elements that can be stored in the
/// dense vector for a better memory efficiency.
pub struct SparseSet<T, I = usize> {
    /// The dense vector that contains the actual values.
    dense: Vec<T>,
    /// The sparse vector that maps the keys to the dense vector indices.
    sparse: Vec<I>,
}

impl<T, I> SparseSet<T, I> {
    /// Creates a new empty [`SparseSet`] instance.
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn new() -> Self {
        Self {
            dense: Vec::new(),
            sparse: Vec::new(),
        }
    }

    /// Returns a slice over the dense vector.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn dense(&self) -> &[T] {
        &self.dense
    }

    /// Returns a mutable slice over the dense vector.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn dense_mut(&mut self) -> &mut [T] {
        &mut self.dense
    }
}

impl<T, I: DenseIndex> SparseSet<T, I> {
    /// Returns an entry in the set for the given key.
    pub fn entry(&mut self, key: usize) -> Entry<T, I> {
        fn grow_for_key<I: DenseIndex>(sparse: &mut Vec<I>, key: usize) {
            if key > isize::MAX as usize {
                capacity_overflow();
            }

            // Reserve space for the new key.
            let additional = unsafe { key.unchecked_sub(sparse.len()).unchecked_add(1) };
            sparse.reserve(additional);

            // Initialize the reserved space with the sentinel value.
            sparse
                .spare_capacity_mut()
                .fill(MaybeUninit::new(I::SENTINEL));
            unsafe { sparse.set_len(sparse.capacity()) };
        }

        if key >= self.sparse.len() {
            grow_for_key(&mut self.sparse, key);
        }

        let dense_index = unsafe { self.sparse.get_unchecked_mut(key) };
        if dense_index.to_usize() == I::SENTINEL.to_usize() {
            self.dense.reserve(1);
            Entry::Vacant(VacantEntry {
                dense_index,
                dense: &mut self.dense,
            })
        } else {
            Entry::Occupied(unsafe { self.dense.get_unchecked_mut(dense_index.to_usize()) })
        }
    }

    /// Inserts a new value in the set.
    ///
    /// If the key already exists in the set, the value is updated and the old value is returned.
    pub fn insert(&mut self, key: usize, value: T) -> Option<T> {
        match self.entry(key) {
            Entry::Occupied(v) => Some(core::mem::replace(v, value)),
            Entry::Vacant(entry) => {
                entry.insert(value);
                None
            }
        }
    }

    /// Gets a value from the set.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get(&self, key: usize) -> Option<&T> {
        self.sparse
            .get(key)
            .filter(|&i| i.to_usize() != I::SENTINEL.to_usize())
            .map(|i| unsafe { self.dense.get_unchecked(i.to_usize()) })
    }

    /// Gets a mutable value from the set.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        self.sparse
            .get(key)
            .filter(|&i| i.to_usize() != I::SENTINEL.to_usize())
            .map(|i| unsafe { self.dense.get_unchecked_mut(i.to_usize()) })
    }

    /// Gets a value from the set without bounds checking.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get_unchecked(&mut self, key: usize) -> &T {
        unsafe {
            let dense_index = *self.sparse.get_unchecked(key);
            self.dense.get_unchecked(dense_index.to_usize())
        }
    }

    /// Gets a mutable value from the set without bounds checking.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        unsafe {
            let dense_index = *self.sparse.get_unchecked(key);
            self.dense.get_unchecked_mut(dense_index.to_usize())
        }
    }
}

impl Default for SparseSet<u8> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}
