//! Type-erased vector.

use core::{alloc::Layout, ptr::NonNull};

use crate::component::DropFn;

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
    pub fn drop_fn(&self) -> Option<DropFn> {
        self.drop_fn
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
