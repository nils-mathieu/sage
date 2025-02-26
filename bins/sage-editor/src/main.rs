//! The present documentation provides information about the internals of Sage's editor.
//!
//! If you are looking for a tutorial on how to use the editor, please refer to the
//! [Sage Documentation](https://sage-engine.org/docs/).

/// The glorious entry point.
pub fn main() {
    let mut app = sage::App::default();
    app.init_schedule(sage::STARTUP_SCHEDULE);
    app.init_schedule(sage::UPDATE_SCHEDULE);
    app.add_system(sage::UPDATE_SCHEDULE, startup);
    sage::run(&mut app);
}

fn startup(mut commands: sage::EventLoopCommands) {}
