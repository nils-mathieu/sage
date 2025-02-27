//! The sage engine framework.
//!
//! This can be used depended on when creating headless components for the Sage ecosystem.

pub use sage_color as color;
pub use sage_core as core;
pub use sage_ui as ui;
pub use sage_wgpu as gfx;
pub use sage_winit as window;

pub use {
    glam::*,
    sage_color::{LinearSrgba, Srgba},
    sage_core::{
        FIXED_UPDATE_SCHEDULE, RENDER_SCHEDULE, TypeUuid, UPDATE_SCHEDULE, Uuid,
        app::{App, Event, EventContext, FromApp},
        schedule::SystemConfig,
        system::{Glob, Query},
    },
    sage_hierarchy::{Children, Parent},
    sage_wgpu::{OutputTarget, PREPARE_FRAME, PendingCommandBuffers, Renderer, SUBMIT_FRAME},
    sage_winit::{
        EventLoopCommands, Window, events as window_events, run,
        winit::{dpi, event::*, keyboard::*, monitor::*, window::WindowAttributes},
    },
};

/// Initializes the default application systems with the provided app.
pub fn init_default(app: &mut App) {
    app.init_schedule(UPDATE_SCHEDULE);
    app.init_schedule(RENDER_SCHEDULE);
    app.init_schedule(FIXED_UPDATE_SCHEDULE);
    sage_ui::initialize(app);
}
