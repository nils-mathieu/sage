//! The core library making the foundation of the Sage engine.
//!
//! It provides the main content management system and ways to interact
//! with it efficiently using parallel processing when possible.

const std = @import("std");

pub const Engine = @import("Engine.zig");
pub const Uuid = @import("Uuid.zig");
pub const Entity = Engine.EntityAllocator.Entity;

test {
    std.testing.refAllDeclsRecursive(@This());
}
