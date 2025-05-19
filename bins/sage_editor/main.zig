const std = @import("std");
const sage = @import("sage");

/// The glorious entry point of the Sage editor.
pub fn main() void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    sage.run(gpa.allocator(), initialize);
}

/// Initializes the editor.
fn initialize(engine: *sage.Engine) void {
    std.log.debug("Initializing the editor...", .{});

    _ = engine;
}
