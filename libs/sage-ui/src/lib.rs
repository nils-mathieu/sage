//! The UI framework of the Sage game engine.

mod ui_node;
pub use self::ui_node::*;

mod brush;
pub use self::brush::*;

use {
    sage_core::{RENDER_SCHEDULE, app::App, schedule::SystemConfig},
    sage_wgpu::{PREPARE_FRAME, SUBMIT_FRAME},
};

pub mod rendering;

/// Initializes the application with the UI framework's systems.
pub fn initialize(app: &mut App) {
    app.init_global::<self::rendering::UiPass>();
    app.add_system(
        RENDER_SCHEDULE,
        SystemConfig::default()
            .tag(PREPARE_FRAME)
            .run_before(SUBMIT_FRAME),
        self::rendering::prepare_frame,
    );
    app.add_system(
        RENDER_SCHEDULE,
        SystemConfig::default()
            .tag(SUBMIT_FRAME)
            .run_after(PREPARE_FRAME),
        self::rendering::submit_frame,
    );
}
