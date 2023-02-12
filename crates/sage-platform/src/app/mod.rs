//! The [`App`] trait.
//!
//! # Application Lifecycle
//!
//! * An application is initially created by the [`run`] function. It takes a [`Config`] object as
//! an input, which defines the initial state of the application.
//!
//! * After the platform has been initialized, the [`App::create`] function is called, giving the
//!   application a chance to initialize its state using the resources provided by the platform.
//!
//! * Once the initialization is complete, the [`App::tick`] function is called repeatedly, but only
//! after all events have been processed.
//!
//! * When the [`App::tick`] function returns [`Tick::Stop`], the application is stops. When it
//!   returns [`Tick::Poll`], the lifecycle repeats; events are processed, and the [`App::tick`]
//!   function is called again. More information in the documentation for [`Tick`].

use crate::device::{DeviceId, Key, MouseButton};

mod ctx;

pub use ctx::*;

/// The result of a call to [`App::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tick {
    /// Indicates that the application should stop.
    Stop,
    /// The application should continue executing.
    ///
    /// When new events are received, the application processes them and the [`App::tick`] function
    /// is called again. If no events are available, the [`App::tick`] function is called
    /// anyway.
    Poll,
    /// The application should continue executing.
    ///
    /// Until a new event is received, the application should block.
    Block,
}

/// Represents the lifecycle of an application.
///
/// More in the [top-level documentation](index.html).
#[allow(unused_variables)]
pub trait App: Sized {
    /// The error type returned by [`App::create`].
    type Error;
    /// An input argument passed to [`App::create`].
    type Args;

    /// Creates a new application.
    fn create(args: Self::Args, ctx: &Ctx) -> Result<Self, Self::Error>;

    /// Called when the application should close itself.
    ///
    /// Note that this function is called when the user wants to close the application, but it is
    /// not guaranteed that the application will stop immediately. The [`App::tick`] function will
    /// be called again, and it is up to the application to decide whether it should stop or not.
    ///
    /// To actually stop the application, the [`App::tick`] function must return [`Tick::Stop`]
    /// rather than [`Tick::Poll`] or [`Tick::Block`].
    fn close_request(&mut self, ctx: &Ctx) {}

    /// Called when the window has been resized.
    fn size(&mut self, ctx: &Ctx, width: u32, height: u32) {}

    /// Called when the window has been moved.
    fn position(&mut self, ctx: &Ctx, x: i32, y: i32) {}

    /// Called when a keyboard key has been pressed.
    fn keyboard_key(&mut self, ctx: &Ctx, dev: DeviceId, key: Key, now_pressed: bool) {}

    /// Called when a mouse button has been pressed.
    fn mouse_button(&mut self, ctx: &Ctx, dev: DeviceId, button: MouseButton, now_pressed: bool) {}

    /// Called when a mouse has moved.
    ///
    /// Note that the `dx` and `dy` values are relative to the previous position of the mouse, and
    /// should not be used to compute the position of a cursor. If you need the position of the
    /// cursor, use the [`App::cursor`] function instead.
    fn mouse_motion(&mut self, ctx: &Ctx, dev: DeviceId, dx: i32, dy: i32) {}

    /// Called when the mouse wheel has been scrolled.
    ///
    /// Note that this event may be generated from a touchpad, and not necessarily from a concrete
    /// mouse wheel.
    fn scroll(&mut self, ctx: &Ctx, dev: DeviceId, dx: f32, dy: f32) {}

    /// Called when the cursor has moved over the window.
    fn cursor(&mut self, ctx: &Ctx, x: u32, y: u32) {}

    /// Called when a text input event has been received.
    ///
    /// `text` will usually contain a single character, but depending on the input device used
    /// (keyboard, IME, etc.), it may contain more than one character.
    fn text(&mut self, ctx: &Ctx, text: &str) {}

    /// Called when the application should close.
    fn tick(&mut self, ctx: &Ctx) -> Tick;
}
