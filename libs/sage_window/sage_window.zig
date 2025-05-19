//! The windowing abstraction that the Sage engine uses to create windows and interact
//! with the operating system's window manager or display server.

const std = @import("std");
const builtin = @import("builtin");

/// The platform-specific implementation.
///
/// Anything past this namespace is platform-specific and
/// must be gated behind the appropriate module.
pub const platform_impl = switch (builtin.os.tag) {
    else => @compileError("unsupported operating system: " ++ @tagName(builtin.os.tag)),
};

/// The configuration passed to the `run` function used to
/// start the event loop.
pub const Config = struct {
    /// The allocator used to allocate memory for the windowing system.
    allocator: std.mem.Allocator,
};

/// Runs an event loop until the application that runs it exits.
///
/// # Thread Safety
///
/// This function must be called from the main thread, and must not be called re-entrantly.
pub fn run(config: Config) void {
    platform_impl.run(config);
}
