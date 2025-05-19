//! The main game framework used by exported games and the editor.
//!
//! Use this over the fully-featured editor when you need a lightweight
//! framework for your game.

const std = @import("std");

const sage_core = @import("sage_core");
const sage_window = @import("sage_window");

pub const Engine = sage_core.Engine;
pub const Uuid = sage_core.Uuid;
pub const Entity = sage_core.Entity;

/// Runs a Sage-powered application to completion.
///
/// # Parameters
///
/// - `allocator`: The allocator to use for memory allocations within the engine.
///
/// - `initialize`: A function that will be called once the engine has been initialized
///   and the application is ready to perform every operation needed to run the game. This includes
///   creating windows, rendering stuff, etc.
///
/// # Valid Usage
///
/// This function must be called on the process's main thread.
///
/// # Control Flow
///
/// On most platforms, this function will only return once the application has exited
/// and the event loop has properly been terminated.
///
/// However, this behavior differs on some platforms:
///
/// - On **iOS**, this function never actually returns and just exists the program by itself
///   once the application has exited.
///
/// - On the web, this function returns instantly and runs the application on the browser's
///   event loop in the background.
///
/// # Remarks
///
/// If you need more control over the application's lifecycle, you can use the `runConfigured`
/// function which does the exact same thing, but takes some additional configuration options
/// if needed.
pub fn run(allocator: std.mem.Allocator, initialize: *const fn (*Engine) void) void {
    var engine = Engine.init(allocator);
    defer engine.deinit();

    initialize(&engine);

    sage_window.run(.{
        .allocator = allocator,
    });
}
