//! Abstracts the underlying operating system and window manager, letting the user to create a
//! surface (usually a window, but not always) on which to render things.
//!
//! Specifically, this crate defines how to represent the state and the transition of popular input
//! devices such as the [keyboard], the mouse, or gamepads.
// TODO: when we have those implemented, add links on "keyboard", "mouse", and "gamepad".

pub mod keyboard;
