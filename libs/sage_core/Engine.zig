//! Contains the complete state of the game. One instance of this struct
//! should be created at the start of the game and passed around.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const ComponentRegistry = @import("Engine/ComponentRegistry.zig");
pub const EntityAllocator = @import("Engine/EntityAllocator.zig");

/// The general purpose allocator used throughout the game.
///
/// This allocator must support general allocations and free operations
/// to make sure that memory can be granularly managed.
allocator: Allocator,
