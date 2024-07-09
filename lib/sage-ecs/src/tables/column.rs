use alloc::alloc::handle_alloc_error;
use core::{alloc::Layout, ptr::NonNull};

use crate::{
    component::{ComponentId, DropFn},
    utility::assert_unchecked,
};

/// A type erased version `Vec<T>` that is used to store a single type of components within a
/// `ColumnStorage`.
///
/// Instead of remembering what kind of data it stores using a genereic parameter (like `Vec<T>`
/// does), [`Column`] stores the memory layout of its elements.
///
/// # Thread Safety
///
/// This type is not thread safe because its content is unknown and may or may not be itself
/// thread safe themselves.
pub struct Column {
    /// The memory layout of the elements stored in this column.
    ///
    /// The `size()` part of this layout is always a multiple of its alignment, ensuring that
    /// aligment remain consistent when multiplying the capacity of the column by the size
    /// of its elements.
    layout: Layout,
    /// The drop function of the elements stored in this column.
    drop_fn: Option<DropFn>,

    /// The data pointer to the first element in the column. This pointer is always correctly
    /// aligned for the type of elements stored in the column, but might not be actually pointing
    /// to a valid allocation.
    ///
    /// Specifically, this pointer points to a valid allocation if and only if either `cap > 0` and
    /// `layout.size() > 0`.
    data: NonNull<u8>,
    /// The number of elements that the column can accommodate without reallocating. This is the
    /// size of the allocation that `data` points to.
    cap: usize,
    /// The number of initialized elements in the column. This is always less than or equal to
    /// `cap`.
    len: usize,
}

impl Column {
    /// Creates a new [`Column`] instance with the given layout and drop function.
    ///
    /// # Parameters
    ///
    /// - `layout`: The memory layout of the elements stored in this column.
    ///
    /// - `drop_fn`: The drop function of the elements stored in this column. If the elements do
    /// not need to be dropped, this can be `None`.
    ///
    /// # Safety
    ///
    /// This function is not directly unsafe, but providing a valid layout/drop_fn pair here is
    /// a requirement to make later unsafe operations correct.
    pub const fn new(layout: Layout, drop_fn: Option<DropFn>) -> Self {
        // If we are storing a zero-sized type, the capacity of the column is infinite (within the
        // allowed memory limit of a `usize`).
        let cap = if layout.size() == 0 { usize::MAX } else { 1 };

        Self {
            layout: pad_layout(layout),
            drop_fn,
            data: NonNull::dangling(),
            cap,
            len: 0,
        }
    }

    /// Returns the memory layout of the elements stored in this column.
    ///
    /// # Remarks
    ///
    /// This function does not necessarily return the same layout that was passed to the
    /// constructor. The column internally stores an *aligned* layout. The size of the layout is
    /// always a multiple of its alignment, whereas the size of the passed layout might not have
    /// been.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Returns the drop function that has been specified for the elements stored in this column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn drop_fn(&self) -> Option<DropFn> {
        self.drop_fn
    }

    /// Returns a raw pointer to the first element in the column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Returns a mutable raw pointer to the first element in the column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// Returns the number of elements initialized in the column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the column contains no elements.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements that the column can accommodate without reallocating.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns the layout of the allocation that the column currently points to.
    pub fn allocated_layout(&self) -> Layout {
        // SAFETY: This computation has been checked when the column was first allocated and has
        // not changed since then.
        unsafe {
            let size = self.layout.size().unchecked_mul(self.cap);
            Layout::from_size_align_unchecked(size, self.layout.align())
        }
    }

    /// Returns the number of additional elements that the column can accommodate without
    /// reallocating.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn spare_capacity(&self) -> usize {
        // SAFETY: By invariant, we know that `len <= cap`.
        unsafe { self.cap.unchecked_sub(self.cap) }
    }

    /// Grows the capacity of the column to at least `new_capacity`.
    ///
    /// # Panics
    ///
    /// This function panics if the column cannot grow to the desired capacity.
    ///
    /// # Safety
    ///
    /// The provided `new_capacity` must be strictly larger than the current capacity of the
    /// column.
    pub unsafe fn grow(&mut self, new_capacity: usize) {
        // SAFETY: Must be upheld by the caller.
        unsafe { assert_unchecked(new_capacity > self.cap) };

        let align = self.layout.align();
        let new_size = new_capacity
            .checked_mul(self.layout.size())
            .filter(|&s| s <= isize::MAX as usize)
            .unwrap_or_else(|| capacity_overflow());

        // SAFETY:
        //  We know that `layout.size()` is non-zero because if it was zero, the capacity would be
        //  `usize::MAX`, which admits no valid value for `new_capacity`.
        //  Additionally, because `new_capacity` must be strictly greater than the capacity, we
        //  know that it cannot be zero. `new_size = new_capacity * layout.size()`, so `new_size`
        //  cannot be zero either.
        unsafe { assert_unchecked(new_size > 0) };

        let new_data = if self.cap == 0 {
            // First allocation for this column.

            // SAFETY: `new_size` is already aligned to `align`, which ensures that
            // `new_size` is a multiple of `align`. We made sure that it has not
            // overflowed `isize::MAX` above.
            let layout = unsafe { Layout::from_size_align_unchecked(new_size, align) };

            // SAFETY: `new_size` is non-zero.
            unsafe { alloc::alloc::alloc(layout) }
        } else {
            // SAFETY: `new_size` is non-zero and the `allocated_layout()` is the original
            // allocation layout.
            unsafe { alloc::alloc::realloc(self.data.as_ptr(), self.allocated_layout(), new_size) }
        };

        if new_data.is_null() {
            let layout = unsafe { Layout::from_size_align_unchecked(new_size, align) };
            handle_alloc_error(layout)
        }

        self.data = unsafe { NonNull::new_unchecked(new_data) };
        self.cap = new_capacity;
    }

    /// Reserves capacity for at least `additional` more elements to be inserted into the column
    /// without reallocation.
    ///
    /// # Panics
    ///
    /// This function panics if it fails to allocate the requested capacity.
    #[cfg_attr(feature = "inline-more", inline)]
    #[track_caller]
    pub fn reserve(&mut self, additional: usize) {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn do_reserve(this: &mut Column, additional: usize) {
            let requested = this
                .len
                .checked_add(additional)
                .unwrap_or_else(|| capacity_overflow());
            let amortized = this.cap.saturating_mul(2);

            // SAFETY: `amortized` is always greater than `this.cap`, except when `this.cap` is
            // is `0`. In that case, the `.max(4)` ensures that the new capacity is still greater
            // than `this.cap`.
            unsafe { this.grow(requested.max(amortized).max(4)) };
        }

        if self.spare_capacity() < additional {
            do_reserve(self, additional)
        }
    }

    /// Gets a raw pointer to one of the elements in the column.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided index is within the capacity of the column. Note
    /// that elements are only initialized up to the length of the column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn get_unchecked(&self, index: usize) -> *const u8 {
        unsafe {
            assert_unchecked(index < self.cap);
            let offset = self.layout.size().unchecked_mul(index);
            self.data.as_ptr().add(offset)
        }
    }

    /// Gets a mutable raw pointer to one of the elements in the column.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided index is within the capacity of the column. Note
    /// that elements are only initialized up to the length of the column.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> *mut u8 {
        unsafe {
            assert_unchecked(index < self.cap);
            let offset = self.layout.size().unchecked_mul(index);
            self.data.as_ptr().add(offset)
        }
    }

    /// Assumes that the `count` first elements past the length of the column have been initialized
    /// and increments the length of the column.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the `count` first elements past the length of the column have
    /// been initialized.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn assume_init_push(&mut self, count: usize) {
        unsafe {
            assert_unchecked(count <= self.spare_capacity());
            self.len = self.len.unchecked_add(count);
        }
    }

    /// Clears the column, dropping all elements.
    ///
    /// # Panics
    ///
    /// This function panics if the drop function provided for the elements in the column panics.
    /// In that case, the column will be completely cleared and the elements that were not yet
    /// dropped will leak.
    pub fn clear(&mut self) {
        let len = self.len;
        self.len = 0;
        if let Some(drop_fn) = self.drop_fn {
            (0..len).for_each(|i| unsafe { drop_fn(self.get_unchecked_mut(i)) });
        }
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        // Make sure that the column is deallocated even if `clear()` panics.
        let layout = self.allocated_layout();
        let data = self.as_mut_ptr();
        let _guard = crate::utility::defer(move || {
            if layout.size() != 0 {
                unsafe { alloc::alloc::dealloc(data, layout) }
            }
        });

        // Remove all elements from the column, ensuring that they are dropped.
        self.clear();
    }
}

/// Pads the provided layout to its alignment.
///
/// This is basically a const version of the `Layout::pad_to_align` method.
const fn pad_layout(l: Layout) -> Layout {
    // SAFETY: A power of two is never zero.
    let align_mask = unsafe { l.align().unchecked_sub(1) };
    // SAFETY: One of the invariants of the `Layout` type is that the size can always be aligned
    // up to the alignment without overflowing `isize::MAX`.
    let size = unsafe { l.size().unchecked_add(align_mask) & !align_mask };

    debug_assert!(size & align_mask == 0);
    debug_assert!(size <= isize::MAX as usize);

    // SAFETY: Alignment has not changed. Size has not overflowed `isize::MAX` by invariant
    // of the input layout.
    unsafe { Layout::from_size_align_unchecked(size, l.align()) }
}

#[inline(never)]
#[track_caller]
#[cold]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

#[cfg(test)]
mod test {
    use core::alloc::Layout;

    use super::pad_layout;

    #[track_caller]
    fn layout(s: usize, a: usize) -> Layout {
        Layout::from_size_align(s, a).unwrap()
    }

    #[test]
    fn test_pad_layout_zst() {
        let l = pad_layout(layout(0, 8));

        assert_eq!(l.size(), 0);
        assert_eq!(l.align(), 8);
    }

    #[test]
    fn test_pad_layout_up() {
        let l = pad_layout(layout(1, 8));

        assert_eq!(l.size(), 8);
        assert_eq!(l.align(), 8);
    }

    #[test]
    fn test_pad_layout_down() {
        let l = pad_layout(layout(7, 8));

        assert_eq!(l.size(), 8);
        assert_eq!(l.align(), 8);
    }

    #[test]
    fn test_pad_layout_same() {
        let l = pad_layout(layout(8, 8));

        assert_eq!(l.size(), 8);
        assert_eq!(l.align(), 8);
    }
}
