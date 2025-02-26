use {
    crate::WinitWindow,
    sage_core::{TypeUuid, Uuid, entities::Component},
    std::sync::Arc,
    winit::dpi::{PhysicalPosition, PhysicalSize},
};

/// A window that the user can interact with.
pub struct Window {
    pub(crate) winit_window: Arc<WinitWindow>,
    pub(crate) surface_size: PhysicalSize<u32>,
    pub(crate) scale_factor: f64,
    pub(crate) pointer_position: Option<PhysicalPosition<f64>>,
    pub(crate) focused: bool,
}

impl Window {
    /// Returns the concrete [`winit`] window object.
    ///
    /// This can be used to interact with the underlying window directly, changing its properties
    /// or querying its state directly using the [`winit`] API.
    ///
    /// Note that a lot of the window's state is cached in the [`Window`] struct already, so it's
    /// usually faster to simply use the methods provided here.
    #[inline(always)]
    pub fn winit_window(&self) -> &WinitWindow {
        &self.winit_window
    }

    /// Returns the current scaling factor of the window.
    #[inline(always)]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Returns the current size of the window's surface, in physical pixels.
    #[inline(always)]
    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.surface_size
    }

    /// Returns the current position of the pointer over the window.
    ///
    /// If the pointer is not over the window, this function returns `None`.
    #[inline(always)]
    pub fn pointer_position(&self) -> Option<PhysicalPosition<f64>> {
        self.pointer_position
    }

    /// Returns whether the window is currently focused or not.
    #[inline(always)]
    pub fn focused(&self) -> bool {
        self.focused
    }
}

unsafe impl TypeUuid for Window {
    const UUID: Uuid = Uuid::from_u128(0x340687371CC878E1463A00938BE6F32D);
}

impl Component for Window {}
