//! A winit-based backend for the Sage game engine.

use {sage_core::app::App, winit::window::WindowAttributes};

pub use winit;

mod app_runner;
pub use self::app_runner::*;

mod window;
pub use self::window::*;

mod event_loop;
pub use self::event_loop::*;

pub mod events;

/// Runs the application to completion.
///
/// # Panics
///
/// This function will panic if any error is encountered during the application's execution.
pub fn run(attrs: WindowAttributes, init_fn: impl 'static + FnOnce(&mut App)) {
    winit::event_loop::EventLoop::new()
        .unwrap_or_else(|err| panic!("Failed to create the `winit` event loop: {err}"))
        .run_app(&mut AppRunner::new(attrs, Box::new(init_fn)))
        .unwrap_or_else(|err| panic!("Failed to run the `winit` event loop: {err}"));
}
