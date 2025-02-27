use {
    sage_core::{
        TypeUuid, Uuid,
        app::{App, AppCell, Global},
        entities::{EntityId, EntityIdAllocator},
        system::{SystemAccess, SystemParam},
    },
    std::ops::{Deref, DerefMut},
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
    ///
    /// The [`Window`] component will be attached to the entity with the specified ID.
    ///
    /// [`Window`]: crate::Window
    pub fn create_window_on(&mut self, target_entity: EntityId, attributes: WindowAttributes) {
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
    /// Creates a new window with the specified attributes.
    ///
    /// # Remarks
    ///
    /// The window will only be created at the end of the current schedule execution.
    pub fn create_window(&mut self, attributes: WindowAttributes) -> EntityId {
        let id = self.id_allocator.reserve_one();
        self.create_window_on(id, attributes);
        id
    }
}

impl Deref for EventLoopCommands<'_> {
    type Target = EventLoopGlobal;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.global
    }
}

impl DerefMut for EventLoopCommands<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.global
    }
}

unsafe impl SystemParam for EventLoopCommands<'_> {
    type State = ();
    type Item<'w> = EventLoopCommands<'w>;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        unsafe {
            EventLoopCommands {
                global: app.global_mut().expect("`EventLoopGlobal` not present"),
                id_allocator: app.get_ref().entities().id_allocator(),
            }
        }
    }
}
