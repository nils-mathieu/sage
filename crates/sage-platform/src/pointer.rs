//! Types to represent the state of a pointer (mouse).
//!
//! Because most platform translate touch events into mouse events, this module also contains types
//! to represent the state of a touch screen.

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// Another extra mouse button.
    Other(u8),
}

/// A unique identifier for a mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MouseId(());
