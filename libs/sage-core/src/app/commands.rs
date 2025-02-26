use {
    crate::app::App,
    std::{alloc::Layout, mem::ManuallyDrop, ptr::NonNull},
};

/// A command that can be executed on an [`App`] once exclusive access is available.
pub trait Command: 'static + Send + Sized {
    /// Executes the command on the provided [`App`].
    fn execute(self, app: &mut App);
}

/// The Vtable of [`RawCommand`].
#[repr(C)]
struct RawCommandVTable<T: ?Sized> {
    /// The size of the command.
    size: usize,
    /// The function pointer to invoke when executing the command.
    ///
    /// `T` might not be aligned if it requires an alignment greater than the machine word.
    execute_fn: unsafe extern "C" fn(*mut T, &mut App),
    /// The function pointer to invoke when dropping the command.
    ///
    /// `T` might not be aligned if it requires an alignment greater than the machine word.
    drop_fn: unsafe extern "C" fn(*mut T),
}

/// Removes the alignment requirement of the inner type.
#[repr(C, packed)]
struct ResetAlign<T>(pub T);

/// An FFI-safe [`Command`].
#[repr(C)]
pub struct RawCommand<T: 'static> {
    /// The vtable for the command.
    vtable: &'static RawCommandVTable<T>,
    /// The command data.
    data: ResetAlign<T>,
}

impl<T: Command> RawCommand<T> {
    /// Creates a new [`RawCommand<T>`] from the provided [`Command`].
    pub fn new(data: T) -> Self {
        unsafe extern "C" fn execute_fn<T: Command>(this: *mut T, app: &mut App) {
            if std::mem::align_of::<T>() > std::mem::align_of::<usize>() {
                unsafe { this.read_unaligned().execute(app) }
            } else {
                unsafe { this.read().execute(app) }
            }
        }

        unsafe extern "C" fn drop_fn<T: Command>(this: *mut T) {
            if std::mem::align_of::<T>() > std::mem::align_of::<usize>() {
                drop(unsafe { this.read_unaligned() });
            } else {
                unsafe { this.drop_in_place() };
            }
        }

        trait VTableProvider {
            const VTABLE: RawCommandVTable<Self>;
        }

        impl<T> VTableProvider for T
        where
            T: Command,
        {
            const VTABLE: RawCommandVTable<Self> = RawCommandVTable {
                size: std::mem::size_of::<RawCommand<T>>(),
                execute_fn: execute_fn::<T>,
                drop_fn: drop_fn::<T>,
            };
        }

        Self {
            vtable: &<T as VTableProvider>::VTABLE,
            data: ResetAlign(data),
        }
    }
}
/// An owned reference to a [`RawCommand<T>`].
///
/// This type will automatically drop the referenced command when it goes out of scope.
pub struct RawCommandRef<'a>(&'a mut RawCommand<()>);

impl RawCommandRef<'_> {
    /// Executes the command on the provided [`App`].
    #[inline(always)]
    pub fn execute(self, app: &mut App) {
        let mut this = ManuallyDrop::new(self);
        unsafe { (this.0.vtable.execute_fn)(&mut this.0.data.0, app) };
    }
}

impl Drop for RawCommandRef<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { (self.0.vtable.drop_fn)(&mut self.0.data.0) }
    }
}

/// A list of [`Command`]s.
pub struct CommandList {
    /// The buffer containing the commands.
    data: NonNull<u8>,
    /// The capacity of the buffer.
    cap: usize,
    /// The current cursor position in the buffer.
    cursor: usize,
}

impl Default for CommandList {
    fn default() -> Self {
        Self {
            data: NonNull::dangling(),
            cap: 0,
            cursor: 0,
        }
    }
}

impl CommandList {
    /// Grows the [`CommandList`] to the provided new capacity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the new capacity is strictly greater than
    /// the current capacity.
    pub unsafe fn grow_unchecked(&mut self, mut new_capacity: usize) {
        const ALIGN: usize = align_of::<usize>();

        let mask = unsafe { ALIGN.unchecked_sub(1) };
        new_capacity = new_capacity
            .checked_add(mask)
            .unwrap_or_else(|| command_list_overflow());

        let new_layout = unsafe { Layout::from_size_align_unchecked(new_capacity, ALIGN) };

        let new_data = if self.cap == 0 {
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            let current_layout = unsafe { Layout::from_size_align_unchecked(self.cap, ALIGN) };
            unsafe { std::alloc::realloc(self.data.as_ptr(), current_layout, new_capacity) }
        };

        if new_data.is_null() {
            std::alloc::handle_alloc_error(new_layout);
        }

        self.data = unsafe { NonNull::new_unchecked(new_data) };
        self.cap = new_capacity;
    }

    /// Allocates enough memory to accommodate a command with the provided layout.
    ///
    /// # Returns
    ///
    /// This function returns a pointer to the allocated memory.
    pub fn allocate(&mut self, layout: Layout) -> *mut u8 {
        // Align the cursor to the layout's requested alignment.
        let mask = unsafe { layout.align().unchecked_sub(1) };
        self.cursor = self
            .cursor
            .checked_add(mask)
            .unwrap_or_else(|| command_list_overflow())
            & !mask;

        let start = self.cursor;

        // Calculate the new cursor position.
        self.cursor = self
            .cursor
            .checked_add(layout.size())
            .unwrap_or_else(|| command_list_overflow());

        if self.cursor > self.cap {
            // We need to grow the buffer.
            unsafe {
                self.grow_unchecked(if self.cap == 0 {
                    layout.size()
                } else {
                    self.cap.max(self.cap * 2).max(self.cursor)
                });
            }
        }

        unsafe { self.data.as_ptr().add(start) }
    }

    /// Pushes a [`RawCommand<T>`] into the list.
    pub fn push_raw<T>(&mut self, command: RawCommand<T>) {
        let p = self
            .allocate(Layout::new::<RawCommand<T>>())
            .cast::<RawCommand<T>>();
        unsafe { p.write(command) };
    }

    /// Pushes a [`Command`] into the list.
    pub fn push(&mut self, command: impl Command) {
        self.push_raw(RawCommand::new(command));
    }

    /// Drains the [`CommandList`], returning an iterator over the commands that were inserted
    /// into the list.
    #[inline]
    pub fn drain(&mut self) -> DrainCommandList<'_> {
        DrainCommandList {
            data: self,
            cursor: 0,
        }
    }

    /// Applies the commands in the list to the provided [`App`].
    #[inline]
    pub fn apply(&mut self, app: &mut App) {
        for command in self.drain() {
            command.execute(app);
        }
    }
}

impl Drop for CommandList {
    fn drop(&mut self) {
        self.drain();
    }
}

/// An iterator that drains the elements of a [`CommandList`].
pub struct DrainCommandList<'a> {
    data: &'a mut CommandList,
    cursor: usize,
}

impl<'a> Iterator for DrainCommandList<'a> {
    type Item = RawCommandRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.data.cursor {
            return None;
        }

        // SAFETY: Invariant of the drain iterator, this is always safe.
        let r = unsafe {
            &mut *self
                .data
                .data
                .as_ptr()
                .add(self.cursor)
                .cast::<RawCommand<()>>()
        };

        // Calculate the next position.
        unsafe {
            self.cursor = self.cursor.unchecked_add(r.vtable.size);
            let mask = align_of::<usize>().unchecked_sub(1);
            self.cursor = self.cursor.unchecked_add(mask) & !mask;
        }

        Some(RawCommandRef(r))
    }
}

impl Drop for DrainCommandList<'_> {
    fn drop(&mut self) {
        // Drop the remaining elements.
        while self.next().is_some() {}
    }
}

#[inline(never)]
#[cold]
fn command_list_overflow() -> ! {
    panic!("Command list overflowed");
}

/// A list of commands to be executed on the [`App`] once exclusive access can be obtained.
pub struct Commands {}
