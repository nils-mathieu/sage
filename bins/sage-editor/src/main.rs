//! The present documentation provides information about the internals of Sage's editor.
//!
//! If you are looking for a tutorial on how to use the editor, please refer to the
//! [Sage Documentation](https://sage-engine.org/docs/).

use sage::ui::rendering::UiRectInstance;

/// The glorious entry point.
pub fn main() {
    sage::run(sage::WindowAttributes::default(), |app| {
        sage::init_default(app);
        app.add_system(
            sage::RENDER_SCHEDULE,
            sage::SystemConfig::default()
                .run_before(sage::SUBMIT_FRAME)
                .run_after(sage::PREPARE_FRAME),
            render_stuff,
        );
        app.add_event_handler(exit_on_escape);
    });
}

fn exit_on_escape(
    event: sage::EventContext<sage::window_events::KeyboardInput>,
    mut commands: sage::EventLoopCommands,
) {
    if event.state.is_pressed() && !event.repeat && event.physical_key == sage::KeyCode::Escape {
        commands.exit();
    }
}

fn render_stuff(mut ui_pass: sage::Glob<&mut sage::ui::rendering::UiPass>) {
    ui_pass.append_rect(UiRectInstance {
        position: sage::Vec2::new(50.0, 50.0),
        size: sage::Vec2::new(120.0, 120.0),
        background_color: sage::LinearSrgba::RED,
        border_color: sage::LinearSrgba::GREEN,
        outline_color: sage::LinearSrgba::BLUE,
        border_radius: [10.0; 4],
        border_thickness: [5.0; 4],
        outline_thickness: 2.0,
        outline_offset: 10.0,
        flags: !0,
        z_index: 0,
    });
}
