use {
    super::{AppCell, Event},
    crate::{
        app::App,
        entities::{ComponentList, EntityId, EntityIdAllocator},
        system::{SystemAccess, SystemParam},
    },
    std::{alloc::Layout, mem::ManuallyDrop, ptr::NonNull, sync::Exclusive},
};

/// A command that can be executed on an [`App`] once exclusive access is available.
pub trait Command: 'static + Send + Sized {
    /// Executes the command on the provided [`App`].
    fn execute(self, app: &mut App);
}

impl<F> Command for F
where
    F: FnOnce(&mut App) + Send + 'static,
{
    #[inline(always)]
    fn execute(self, app: &mut App) {
        self(app);
    }
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

/// The alignment of the [`RawCommand`] type.
const ALIGN: usize = align_of::<usize>();

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
            if align_of::<T>() > ALIGN {
                unsafe { this.read_unaligned().execute(app) }
            } else {
                unsafe { this.read().execute(app) }
            }
        }

        unsafe extern "C" fn drop_fn<T: Command>(this: *mut T) {
            if align_of::<T>() > ALIGN {
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

unsafe impl Send for CommandList {}

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
            let mask = ALIGN.unchecked_sub(1);
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
pub struct Commands<'a> {
    /// The list of commands that have been accumulated.
    list: &'a mut CommandList,
    /// The entity ID allocator.
    id_allocator: &'a EntityIdAllocator,
}

impl<'w> Commands<'w> {
    /// Pushes the provided command to the list.
    #[inline]
    pub fn append(&mut self, command: impl Command) {
        self.list.push(command);
    }

    /// Spawns an empty entity and returns a [`EntityCommands`] instance that can be used to
    /// spawn components on the entity.
    pub fn spawn_empty(&mut self) -> EntityCommands<'_, 'w> {
        let target = self.id_allocator.reserve_one();
        EntityCommands {
            commands: self,
            target,
        }
    }

    /// Spawns an entity with the provided list of components.
    pub fn spawn(&mut self, components: impl ComponentList) -> EntityCommands<'_, 'w> {
        let mut entity = self.spawn_empty();
        entity.insert(components);
        entity
    }

    /// Triggers an event with the provided data.
    pub fn trigger_event(&mut self, target: EntityId, mut event: impl Event) {
        self.append(move |app: &mut App| app.trigger_event(target, &mut event))
    }

    /// Despawns the provided entity.
    pub fn despawn(&mut self, entity: EntityId) {
        self.append(move |app: &mut App| app.despawn(entity));
    }
}

unsafe impl SystemParam for Commands<'_> {
    type State = Exclusive<CommandList>;
    type Item<'w> = Commands<'w>;

    #[inline]
    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {
        Exclusive::default()
    }

    #[inline]
    unsafe fn apply_deferred(state: &mut Self::State, app: &mut App) {
        state.get_mut().apply(app);
    }

    #[inline]
    unsafe fn fetch<'w>(state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        Commands {
            id_allocator: unsafe { app.get_ref().entities().id_allocator() },
            list: state.get_mut(),
        }
    }
}

/// Like [`Commands`], but scoped to a specific entity.
pub struct EntityCommands<'cmd, 'w> {
    commands: &'cmd mut Commands<'w>,
    target: EntityId,
}

impl EntityCommands<'_, '_> {
    /// Returns the ID of the entity that will be spawned.
    #[inline]
    pub fn id(&self) -> EntityId {
        self.target
    }

    /// Inserts components into the entity.
    pub fn insert(&mut self, list: impl ComponentList) {
        let target = self.target;
        self.commands
            .append(move |app: &mut App| app.entity_mut(target).insert(list));
    }
}
