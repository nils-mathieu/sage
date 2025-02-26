//! A winit-based backend for the Sage game engine.

use sage_core::{Uuid, app::App};

pub use winit::{
    dpi,
    window::{Window as WinitWindow, WindowAttributes},
};

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
pub fn run(app: &mut App) {
    winit::event_loop::EventLoop::new()
        .unwrap_or_else(|err| panic!("Failed to create the `winit` event loop: {err}"))
        .run_app(&mut AppRunner::new(app))
        .unwrap_or_else(|err| panic!("Failed to run the `winit` event loop: {err}"));
}

/// The UUID of the startup schedule.
pub const STARTUP_SCHEDULE: Uuid = Uuid::from_u128(0x0A863285D65B9349249E8C17B04BC4D2);

/// The UUID of the update schedule.
pub const UPDATE_SCHEDULE: Uuid = Uuid::from_u128(0x3C1391F7321FA09E9BF1F0E50F9694F3);
