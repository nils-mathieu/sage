//! Defines the types required to represent the state of input devices.
//!
//! # Physical And Symbolic Keys
//!
//! You will notice that this module defines two separate types to represent keyboard keys.
//!
//! First, there is the [`Key`] enumeration. It is meant to represent the symblic meaning of a key
//! rather than its physical location on the keyboard. In other words, [`Key`] depends on the
//! user's *keyboard layout*. It was once a physical key-code, but has been translated into a
//! symbolic key by the operating system or window manager.
//!
//! On the other hand, we have the [`ScanCode`] struct. Rather than a symbol, a scan-code
//! represents a concrete keyboard key. It is a (rough) representation of the data sent by the
//! keyboard down the wire.
//!
//! What should you choose then? If the physical location of a key matters more than its logical
//! meaning, you should use the [`ScanCode`] type. Conversly, if you need the know what key the
//! user *meant* to press, then the [`Key`] enumeration is a better choice.
//!
//! # Touch Screen And Mouse
//!
//! Most operating systems and window managers will translate some touch screen events into mouse
//! events for compatiblity reasons. For example, a single tap on a touch screen will be translated
//! into a mouse click. When a platform does not emulate automatically this behavior, this crate
//! will try to emulate it itself for consistency between implementations.

mod keyboard;
mod pointer;

pub use keyboard::*;
pub use pointer::*;

/// A unique identifier for an input device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum DeviceId {
    #[cfg(target_os = "windows")]
    Windows(crate::windows::DeviceId),
}
