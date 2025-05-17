//! Defines the common type used by all runtime backends of the
//! Sage engine.

pub const Key = @import("Key.zig").Key;

/// The ID of a device.
pub const DeviceId = usize;

/// An event indicating that a keyboard key was pressed
/// or released.
pub const KeyEvent = struct {
    /// The ID of the device that generated the event.
    device_id: DeviceId,
    /// The physical key.
    key: Key,
    /// The raw scan-code associated with the key.
    ///
    /// This field is platform-specific and is not guaranteed to follow
    /// any consistent pattern of standard.
    scancode: u32,
    /// The action that was taken on the key.
    action: Action,

    /// An action that can be applied to a key.
    pub const Action = enum {
        /// The key was pressed.
        pressed,
        /// The was released.
        released,
        /// The key was pressed, while it was already
        /// pressed.
        repeated,
    };
};

/// An event indicating that a pointer was moved over a window.
pub const PointerMotionEvent = struct {
    /// The ID of the device that generated the event.
    device_id: DeviceId,
    /// The X coordinate of the cursor, relative to the window.
    x: f64,
    /// The Y coordinate of the cursor, relative to the window.
    y: f64,
};

/// A mouse button.
pub const MouseButton = enum(u8) {
    left,
    middle,
    right,
    backward,
    forward,
    _,
};

/// An action that can be taken on a button.
pub const ButtonAction = enum {
    /// The button was pressed.
    pressed,
    /// The button was released.
    released,
};

/// An event indicating that a mouse button has been used.
///
/// Note that this event might be generated when using a mouse-like device
/// like a pen or trackpad.
pub const MouseButtonEvent = struct {
    /// The ID of the device.
    device_id: DeviceId,
    /// The button that was pressed or released.
    button: MouseButton,
    /// Whether the button was pressed or released.
    action: ButtonAction,
    /// The X coordinate of the cursor at the time of the event, relative
    /// to the window.
    x: f64,
    /// The Y coordinate of the cursor at the time of the event, relative
    /// to the window.
    y: f64,
};

/// An event indicating that a mouse was moved.
pub const MouseMotionEvent = struct {
    /// The ID of the device that generated the event.
    device_id: DeviceId,
    /// The relative horizontal motion of the mouse, relative to the window.
    dx: f64,
    /// The relative vertical motion of the mouse, relative to the window.
    dy: f64,
};

/// An event indicating that the mouse wheel has moved.
pub const MouseWheelEvent = struct {
    /// The ID of the device that generated the event.
    device_id: DeviceId,
    /// The relative horizontal motion of the mouse wheel.
    ///
    /// The unit of this field depends on the `unit` field.
    dx: f64,
    /// The relative vertical motion of the mouse wheel.
    ///
    /// The unit of this field depends on the `unit` field.
    dy: f64,
    /// The unit of the motion.
    unit: Unit,
    /// The X position of the cursor at the time of the event, relative
    /// to the window.
    x: f64,
    /// The Y position of the cursor at the time of the event, relative
    /// to the window.
    y: f64,

    /// The unit of the motion.
    pub const Unit = enum {
        /// The unit is pixels.
        pixels,
        /// The unit is lines.
        lines,
        /// The unit is pages.
        pages,
    };
};

/// An event indicating that the sclae factor of a window has changed.
pub const ScaleFactorChanged = struct {
    /// The new scale factor of the window.
    scale_factor: f64,
};

/// An event indicating that a window's surface area has changed.
pub const WindowResizedEvent = struct {
    /// The new width of the window.
    ///
    /// This corresponds to the width of the window's surface area.
    width: u32,
    /// The new height of the window.
    ///
    /// This corresponds to the height of the window's surface area.
    height: u32,
};
