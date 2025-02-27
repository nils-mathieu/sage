//! Events that the application can receive.

use {
    sage_core::{TypeUuid, Uuid, app::Event},
    std::ops::{Deref, DerefMut},
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{DeviceId, InnerSizeWriter},
    },
};

/// An event that is sent to a window when it is requested to close itself.
///
/// The action can be prevented by calling [`CloseRequested::prevent`].
#[derive(Default)]
pub struct CloseRequested {
    /// Whether the close requested has been prevented or not.
    prevented: bool,
}

impl CloseRequested {
    /// Prevents the window from closing.
    #[inline(always)]
    pub fn prevent(&mut self) {
        self.prevented = true;
    }

    /// Returns whether the close request has been prevented or not.
    #[inline(always)]
    pub fn is_prevented(&self) -> bool {
        self.prevented
    }
}

unsafe impl TypeUuid for CloseRequested {
    const UUID: Uuid = Uuid::from_u128(0x0C15E82B0D8789100E0A018B7FFF2F47);
}

impl Event for CloseRequested {
    type Propagation = ();
}

/// An **event** indicating that the window's surface area has been resized.
pub struct SurfaceResized(pub PhysicalSize<u32>);

impl Deref for SurfaceResized {
    type Target = PhysicalSize<u32>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl TypeUuid for SurfaceResized {
    const UUID: Uuid = Uuid::from_u128(0x571BA0ED4B65F687D11984630AE4A4A5);
}

impl Event for SurfaceResized {
    type Propagation = ();
}

/// An **event** indicating that the window has been moved.
pub struct Moved(pub PhysicalSize<u32>);

unsafe impl TypeUuid for Moved {
    const UUID: Uuid = Uuid::from_u128(0x01833DF513C4BF963087279D48058DEC);
}

impl Event for Moved {
    type Propagation = ();
}

/// An **event** indicating that the window's scale factor has changed.
pub struct ScaleFactorChanged {
    /// The new scale factor of the window.
    pub scale_factor: f64,
    /// An object that can be used to modify the size of the window during scale changes.
    pub inner_size_writer: InnerSizeWriter,
}

unsafe impl TypeUuid for ScaleFactorChanged {
    const UUID: Uuid = Uuid::from_u128(0x30E8744E07D2E425020587A383AE7EAA);
}

impl Event for ScaleFactorChanged {
    type Propagation = ();
}

/// An **event** indicating that the pointer has moved.
pub struct PointerMoved {
    /// The new position of the pointer.
    pub position: PhysicalPosition<f64>,
    /// The device ID of the pointer.
    pub device_id: DeviceId,
}

unsafe impl TypeUuid for PointerMoved {
    const UUID: Uuid = Uuid::from_u128(0x6F49F50864240B7A30FBF489EDBB4257);
}

impl Event for PointerMoved {
    type Propagation = ();
}

/// An **event** indicating that the pointer has entered the window's surface area.
pub struct PointerEntered {
    /// The position of the pointer.
    pub device_id: DeviceId,
}

unsafe impl TypeUuid for PointerEntered {
    const UUID: Uuid = Uuid::from_u128(0x9569935C80EB4C71012AA1B8675C4F27);
}

impl Event for PointerEntered {
    type Propagation = ();
}

/// An **event** indicating that the pointer has left the window's surface area.
pub struct PointerLeft {
    /// The position of the pointer.
    pub device_id: DeviceId,
}

unsafe impl TypeUuid for PointerLeft {
    const UUID: Uuid = Uuid::from_u128(0x6C7DD19D9470C4A24E6D32957A929502);
}

impl Event for PointerLeft {
    type Propagation = ();
}

/// An **event** indicating that the window has been focused or unfocused.
pub struct Focused(pub bool);

unsafe impl TypeUuid for Focused {
    const UUID: Uuid = Uuid::from_u128(0x1D1CD69CBEE6109FA772246E4A9811F8);
}

impl Event for Focused {
    type Propagation = ();
}

/// An **event** indicating that a keyboard key has been pressed or released.
pub struct KeyboardInput {
    /// The inner winit event.
    pub inner: winit::event::KeyEvent,

    /// The device ID of the keyboard.
    pub device_id: DeviceId,

    /// Whether the event was synthesized by `winit` to ensure platform compatibility.
    pub is_synthetic: bool,
}

impl Deref for KeyboardInput {
    type Target = winit::event::KeyEvent;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for KeyboardInput {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

unsafe impl TypeUuid for KeyboardInput {
    const UUID: Uuid = Uuid::from_u128(0xA12EAE724F3DB35A6FE1C83686EDF398);
}

impl Event for KeyboardInput {
    type Propagation = ();
}
