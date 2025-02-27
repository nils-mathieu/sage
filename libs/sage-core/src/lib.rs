//! Defines the barebones core functionality of the Sage game engine.
//!
//! This does not include much, appart from state management.

#![feature(const_type_name)]
#![feature(nonnull_provenance)]
#![feature(exclusive_wrapper)]
#![feature(slice_partition_dedup)]

pub mod app;

mod uuid;
pub use self::uuid::*;

mod opaque_ptr;
pub use self::opaque_ptr::*;

pub mod entities;
pub mod schedule;
pub mod system;

/// The UUID of the update schedule.
///
/// The update schedule is executed any time the window needs to be redrawn. This meams that it is
/// heavily dependent on the refresh rate of the monitor (or if you're not continuously redrawing,
/// this will only run when the window is resized or the [`Window::request_redraw`] method is
/// called).
pub const UPDATE_SCHEDULE: Uuid = Uuid::from_u128(0x3C1391F7321FA09E9BF1F0E50F9694F3);

/// The UUID of the render schedule.
///
/// This schedule is executed in order to render stuff to the screen. It usually runs after
/// the `UPDATE_SCHEDULE`, but potentially on a different thread.
pub const RENDER_SCHEDULE: Uuid = Uuid::from_u128(0x54e3cde8ae8f72b74c11cba46ad2d491);

/// The UUID of the fixed update schedule.
///
/// This schedule is executed at a fixed rate, regardless of the refresh rate of the monitor, or
/// whether the window is being redrawn or not. This means that any logic that needs to be executed
/// at a fixed rate.
///
/// Physics calculation and time-sensitive logic should generally run here.
pub const FIXED_UPDATE_SCHEDULE: Uuid = Uuid::from_u128(0x97d6c77247982377234523b8f888cd7f);
