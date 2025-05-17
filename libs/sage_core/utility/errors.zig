const std = @import("std");

/// Panics with a message indicating that the system has run out of memory.
pub fn oom() noreturn {
    std.debug.panic("The system has run out of memory", .{});
}
