use {
    sage_core::{
        TypeUuid, Uuid,
        app::{App, Global},
        entities::{EntityId, EntityIdAllocator},
        system::{SystemAccess, SystemParam},
    },
    winit::window::WindowAttributes,
};

/// Global resource for the event loop.
#[derive(Default)]
pub struct EventLoopGlobal {
    /// Whether the event loop should exit as soon as possible.
    exit_requested: bool,
    /// A collection of windows waiting to be created.
    pending_windows: Vec<(EntityId, WindowAttributes)>,
}

impl EventLoopGlobal {
    /// Requests the event loop to close itself when it can.
    #[inline(always)]
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    /// Returns whether event loop exit has been requested.
    #[inline(always)]
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Queues a window for creation.
    pub fn create_window(&mut self, target_entity: EntityId, attributes: WindowAttributes) {
        self.pending_windows.push((target_entity, attributes));
    }

    /// Removes the pending windows from the global resources.
    pub(crate) fn take_pending_windows(&mut self) -> Option<Vec<(EntityId, WindowAttributes)>> {
        if self.pending_windows.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.pending_windows))
        }
    }
}

unsafe impl TypeUuid for EventLoopGlobal {
    const UUID: Uuid = Uuid::from_u128(0x418bfdb8869365fd7de601bec909a7c1);
}

impl Global for EventLoopGlobal {}

/// Commands to interact with the event loop without having to work directly
/// with the [`EventLoopGlobal`] resource.
pub struct EventLoopCommands<'w> {
    global: &'w mut EventLoopGlobal,
    id_allocator: &'w EntityIdAllocator,
}

impl EventLoopCommands<'_> {
    /// Requests the event loop to exit as soon as possible.
    ///
    /// In most cases, the event loop will exit at the end of the current schedule execution.
    #[inline]
    pub fn exit(&mut self) {
        self.global.exit();
    }

    /// Creates a new window with the specified attributes.
    ///
    /// # Remarks
    ///
    /// The window will only be created at the end of the current schedule execution.
    pub fn create_window(&mut self, attributes: WindowAttributes) -> EntityId {
        let id = self.id_allocator.reserve_one();
        self.global.create_window(id, attributes);
        id
    }
}

unsafe impl SystemParam for EventLoopCommands<'_> {
    type State = ();
    type Item<'w> = EventLoopCommands<'w>;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: &'w App) -> Self::Item<'w> {
        unsafe {
            EventLoopCommands {
                global: app
                    .globals()
                    .get_raw(EventLoopGlobal::UUID)
                    .expect("`EventLoopGlobal` not present")
                    .data()
                    .as_mut(),
                id_allocator: app.entities().id_allocator(),
            }
        }
    }
}
