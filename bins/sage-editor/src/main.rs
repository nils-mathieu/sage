//! The present documentation provides information about the internals of Sage's editor.
//!
//! If you are looking for a tutorial on how to use the editor, please refer to the
//! [Sage Documentation](https://sage-engine.org/docs/).

/// The glorious entry point.
pub fn main() {
    sage::run(sage::WindowAttributes::default(), |app| {
        sage::init_default(app);
        app.add_event_handler(exit_on_escape);

        app.spawn((
            sage::ui::UiNodeMetrics {
                z_index: 0,
                size: sage::UVec2::new(100, 100),
                position: sage::IVec2::new(100, 100),
                baseline: sage::IVec2::ZERO,
            },
            sage::ui::Background {
                color: sage::Srgba8::RED,
                corner_radius: [2000.0; 4],
                border_thickness: 2.0,
                border_color: sage::Srgba8::GREEN,
            },
        ));
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
