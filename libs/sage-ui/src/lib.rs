//! The UI framework of the Sage game engine.

use {
    sage_core::{RENDER_SCHEDULE, app::App, schedule::SystemConfig},
    sage_wgpu::{PREPARE_FRAME, SUBMIT_FRAME},
};

pub use cosmic_text;

mod ui_node;
pub use self::ui_node::*;

mod fonts;
pub use self::fonts::*;

mod background;
pub use self::background::*;

pub mod rendering;

/// Initializes the application with the UI framework's systems.
pub fn initialize(app: &mut App) {
    app.init_global::<Fonts>();
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
    app.add_system(
        RENDER_SCHEDULE,
        SystemConfig::default()
            .run_before(SUBMIT_FRAME)
            .run_after(PREPARE_FRAME),
        self::background::draw_backgrounds,
    );
    app.add_event_handler(self::rendering::update_view_resolution);
}
