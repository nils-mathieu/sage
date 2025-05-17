//! This library provides the core functionality of the Sage
//! game engine, including a modular architecture for building
//! and running real-time applications.
//!
//! It provides ways to store data in a efficient way and how to
//! manage it effectively during runtime.

pub const Engine = @import("Engine.zig");
pub const Uuid = @import("Uuid.zig");

pub const Entity = Engine.EntityAllocator.Entity;

test {
    @import("std").testing.refAllDeclsRecursive(@This());
}
