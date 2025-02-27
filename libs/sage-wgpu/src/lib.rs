//! A wgpu-based rendering backend for the Sage game engine.

pub use wgpu;

mod renderer;
pub use self::renderer::*;

mod globals;
pub use self::globals::*;

use sage_core::Uuid;

/// A system that that prepares the frame for rendering.
///
/// Should be the first thing to run in the rendering schedule.
pub const PREPARE_FRAME: Uuid = Uuid::from_u128(0x0e99cb3dcdd9c5d01b7ee3f40f4751a6);

/// A system that submits the frame for rendering.
///
/// Should be the last thing to run in the rendering schedule.
pub const SUBMIT_FRAME: Uuid = Uuid::from_u128(0x976077d3091ef184cd656b9e63d8d847);
