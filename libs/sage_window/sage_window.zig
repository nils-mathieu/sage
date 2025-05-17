//! A runtime for the Sage game engine.

const sage_core = @import("sage_core");
const Engine = sage_core.Engine;
const std = @import("std");

/// Runs the an application to completion.
pub fn run(initialize: *const fn (engine: *Engine) void) void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    var engine = Engine.init(gpa.allocator());
    defer engine.deinit();

    initialize(&engine);
}
