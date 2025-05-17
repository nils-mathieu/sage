const std = @import("std");
const sage = @import("sage");

/// The glorious entry point of the Sage editor.
pub fn main() void {
    sage.run(initialize);
}

/// Initializes the editor.
fn initialize(engine: *sage.Engine) void {
    std.log.debug("Initializing the editor...", .{});

    _ = engine;
}
