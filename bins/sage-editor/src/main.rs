//! The present documentation provides information about the internals of Sage's editor.
//!
//! If you are looking for a tutorial on how to use the editor, please refer to the
//! [Sage Documentation](https://sage-engine.org/docs/).

use sage::ui::{cosmic_text, rendering::UiRectInstance};

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

fn render_stuff(
    mut ui_pass: sage::Glob<&mut sage::ui::rendering::UiPass>,
    mut fonts: sage::Glob<&mut sage::ui::Fonts>,
    renderer: sage::Glob<&sage::Renderer>,
) {
    ui_pass.append_rect(UiRectInstance {
        position: sage::Vec2::new(50.0, 50.0),
        size: sage::Vec2::new(500.0, 500.0),
        color: sage::LinearSrgba::RED,
        corner_radius: sage::Vec4::new(10.0, 20.0, 30.0, 40.0),
        border_size: 2.0,
        z_index: 0,
        _padding: [0, 0],
    });

    let mut text = cosmic_text::Buffer::new(
        fonts.as_font_system_mut(),
        cosmic_text::Metrics {
            font_size: 64.0,
            line_height: 64.0,
        },
    );

    text.set_text(
        fonts.as_font_system_mut(),
        "Hello, world!",
        cosmic_text::Attrs::new(),
        cosmic_text::Shaping::Advanced,
    );

    text.shape_until_scroll(fonts.as_font_system_mut(), false);

    ui_pass.append_text_buffer(
        &renderer,
        fonts.as_font_system_mut(),
        sage::Vec2::new(120.0, 120.0),
        1.0,
        0,
        sage::Srgba8::CYAN,
        &text,
    );
}
