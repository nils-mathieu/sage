use {
    crate::{entities::ComponentLayout, opaque_ptr::OpaquePtr},
    std::alloc::Layout,
};

/// A [`Vec`] that stores elements of a compile-time unknown type.
///
/// The referenced data is known to be `Send` and `Sync`.
pub struct ComponentVec {
    /// A pointer to the vector's data buffer.
    data: OpaquePtr,

    /// The number of elements that the buffer can accommodate for.
    cap: usize,
    /// The number of elements that the buffer currently holds.
    len: usize,

    /// The layout of the elements stored in the buffer.
    ///
    /// The memory layout stored here is padded to its alignment, ensuring that the associated size
    /// is actually the array stride used to access the elements.
    layout: ComponentLayout,
}

impl ComponentVec {
    /// Creates a new [`UntypedVec`] with the provided layout.
    ///
    pub fn new(mut layout: ComponentLayout) -> Self {
        layout.memory = layout.memory.pad_to_align();

        Self {
            data: OpaquePtr::dangling_for(layout.memory),
            cap: if layout.memory.size() == 0 {
                usize::MAX
            } else {
                0
            },
            len: 0,
            layout,
        }
    }

    /// Returns a pointer to the element at `index`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the index is less than the vector's capacity.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> OpaquePtr {
        let offset = unsafe { self.layout.memory.size().unchecked_mul(index) };
        self.data.byte_add(offset)
    }

    /// Returns the current memory layout of this vector's backing allocation.
    pub fn current_layout(&self) -> Layout {
        // SAFETY: We used this layout to allocate for the vector's data, ensuring that the
        // operation is safe.
        unsafe {
            let capacity_in_bytes = self.cap.unchecked_mul(self.layout.memory.size());
            Layout::from_size_align_unchecked(capacity_in_bytes, self.layout.memory.align())
        }
    }

    /// Returns a pointer to the vector's data buffer.
    #[inline(always)]
    pub fn as_ptr(&self) -> OpaquePtr {
        self.data
    }

    /// Grows the vector's capacity to `new_capacity`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `new_capacity` is strictly larger than the vector's current
    /// capacity.
    pub unsafe fn grow_unchecked(&mut self, new_capacity: usize) {
        let new_capacity_in_bytes = new_capacity
            .checked_mul(self.layout.memory.size())
            .unwrap_or_else(|| capacity_overflow());

        // SAFETY: We know that `self.layout.memory.size()` is already a multiple of `align`,
        // meaning that rounding up won't overflow (it won't change at all).
        let new_layout = unsafe {
            Layout::from_size_align_unchecked(new_capacity_in_bytes, self.layout.memory.align())
        };

        let new_data = if self.cap == 0 {
            // SAFETY: When the size elements is zero, the vector has a capacity of `usize::MAX`,
            // which mean that `new_capacity` has no possible values. The function cannot be called
            // safely.
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            let current_layout = self.current_layout();
            unsafe { std::alloc::realloc(self.data.as_ptr(), current_layout, new_layout.size()) }
        };

        if new_data.is_null() {
            std::alloc::handle_alloc_error(new_layout);
        }

        // SAFETY: We just checked the return value of `alloc`.
        self.data = unsafe { OpaquePtr::from_raw(new_data) };
        self.cap = new_capacity;
    }

    /// Grows the capacity of the vector using the default growth function.
    pub fn grow_once(&mut self) {
        let new_cap = if self.cap == 0 {
            1
        } else {
            self.cap
                .checked_mul(2)
                .unwrap_or_else(|| capacity_overflow())
        };

        // SAFETY: `new_cap > self.cap`.
        unsafe { self.grow_unchecked(new_cap) };
    }

    /// Reserves space for at least one additional element in the vector.
    #[inline]
    pub fn reserve_one(&mut self) {
        if self.len == self.cap {
            self.grow_once()
        }
    }

    /// Pushes a new element into the vector.
    ///
    /// The element is copied from the provided `src`, moving it into the vector.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the vector has the capacity required to
    ///   store the new element.
    ///
    /// - The caller must ensure that the memory pointed to by `src` follows the layout used to
    ///   create the vector in the first place.
    pub unsafe fn push_assume_capacity(&mut self, src: OpaquePtr) {
        // SAFETY: `len < cap`.
        unsafe {
            let dst = self.get_unchecked(self.len);
            std::ptr::copy_nonoverlapping(
                src.as_ptr::<u8>(),
                dst.as_ptr::<u8>(),
                self.layout.memory.size(),
            );
        }

        // SAFETY: `len < cap <= usize::MAX`.
        self.len = unsafe { self.len.unchecked_add(1) };
    }

    /// Removes the element at the provided `index` from the vector and replaces
    /// it with the last element.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided index is within bounds.
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) {
        unsafe {
            if let Some(drop_fn) = self.layout.drop_fn {
                drop_fn(self.get_unchecked(index));
            }

            let new_len = self.len.unchecked_sub(1);

            std::ptr::copy(
                self.get_unchecked(new_len).as_ptr::<u8>(),
                self.get_unchecked(index).as_ptr::<u8>(),
                self.layout.memory.size(),
            );

            self.len = new_len;
        }
    }
}

impl Drop for ComponentVec {
    fn drop(&mut self) {
        struct Guard {
            layout: Layout,
            data: OpaquePtr,
        }

        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { std::alloc::dealloc(self.data.as_ptr(), self.layout) };
            }
        }

        let _guard = Guard {
            layout: self.current_layout(),
            data: self.data,
        };

        if let Some(drop_fn) = self.layout.drop_fn {
            for i in 0..self.len {
                unsafe { drop_fn(self.get_unchecked(i)) };
            }
        }
    }
}

#[inline(never)]
#[cold]
#[track_caller]
fn capacity_overflow() -> ! {
    panic!("Too many entities")
}
