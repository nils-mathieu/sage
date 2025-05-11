//! Contains information about the components used in the game.
//!
//! # What is a component?
//!
//! A component is a granular piece of information that can be managed
//! by the game engine.
//!
//! Components are attached to a particular entity, and each entity can
//! have at most one instance of a component.
//!
//! # Registering components
//!
//! Components can be registered manually through the `ComponentRegistry`
//! by calling the `registerZigType` method when a regular Zig type is
//! available.
//!
//! Otherwise, it is possible to provide custom component implementations
//! using the `registerComponent` method.

const std = @import("std");
const Alignment = std.mem.Alignment;
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const Engine = @import("../Engine.zig");
const Entity = @import("EntityAllocator.zig").Entity;
const Uuid = @import("../Uuid.zig");

const Self = @This();

/// Stores information about a component.
pub const ComponentInfo = struct {
    /// The name of the component, used for debugging purposes.
    ///
    /// The string should exist in static memory because it will *not*
    /// be freed.
    name: []const u8,

    /// The size of the component, in bytes. This is a multiple
    /// of the component's alignment requirement, ensuring that it
    /// is possible to step through a list of this component without
    /// having to worry about alignment issues.
    size: usize,
    /// The alignment requirement of the component, in bytes.
    alignment: Alignment,

    /// A deinitialization function for the component.
    ///
    /// This function is meant to be called when the component is
    /// no longer used and needs to release any resources it holds.
    ///
    /// # Parameters
    ///
    /// - `self`: The component instance being removed.
    ///
    /// - `engine`: The engine in which the component was registered.
    deinit: ?*const fn (self: *anyopaque, engine: *Engine) void,

    /// A function to be called when the component is inserted into an entity.
    ///
    /// The function is triggered *after* the component has been inserted into the entity.
    ///
    /// # Parameters
    ///
    /// - `self`: The component instance being inserted.
    ///
    /// - `engine`: The engine in which the component was registered.
    ///
    /// - `entity`: The entity into which the component is being inserted.
    onInsertHook: ?*const fn (self: *anyopaque, engine: *Engine, entity: Entity) void,

    /// A function to be called when the component is removed from an entity.
    ///
    /// The function is triggered *before* the component is removed from the entity.
    ///
    /// # Parameters
    ///
    /// - `self`: The component instance being removed.
    ///
    /// - `engine`: The engine in which the component was registered.
    ///
    /// - `entity`: The entity from which the component is being removed.
    onRemoveHook: ?*const fn (self: *anyopaque, engine: *Engine, entity: Entity) void,

    /// Returns the `ComponentInfo` associated with the provided Zig type.
    ///
    /// See `registerZigType` for more information.
    pub fn of(comptime T: type) ComponentInfo {
        const Fns = struct {
            fn deinit(self: *anyopaque, engine: *Engine) void {
                if (!@hasDecl(T, "sageDeinit")) return;
                T.sageDeinit(@alignCast(@ptrCast(self)), engine);
            }

            fn onInsertHook(self: *anyopaque, engine: *Engine, entity: Entity) void {
                if (!@hasDecl(T, "sageOnInsertHook")) return;
                T.sageOnInsertHook(@alignCast(@ptrCast(self)), engine, entity);
            }

            fn onRemoveHook(self: *anyopaque, engine: *Engine, entity: Entity) void {
                if (!@hasDecl(T, "sageOnRemoveHook")) return;
                T.sageOnRemoveHook(@alignCast(@ptrCast(self)), engine, entity);
            }
        };

        return ComponentInfo{
            .name = if (@hasDecl(T, "sage_name")) T.sage_name else @typeName(T),
            .size = @sizeOf(T),
            .alignment = Alignment.of(T),
            .deinit = if (@hasDecl(T, "sageDeinit")) Fns.deinit else null,
            .onRemoveHook = if (@hasDecl(T, "sageOnRemoveHook")) Fns.onRemoveHook else null,
            .onInsertHook = if (@hasDecl(T, "sageOnInsertHook")) Fns.onInsertHook else null,
        };
    }
};

test "ComponentInfo.of empty" {
    const TestComponent = struct {
        data: u32,
    };

    const info = ComponentInfo.of(TestComponent);
    try std.testing.expectEqual(4, info.size);
    try std.testing.expectEqual(Alignment.@"4", info.alignment);
    try std.testing.expectEqual(null, info.deinit);
    try std.testing.expectEqual(null, info.onRemoveHook);
    try std.testing.expectEqual(null, info.onInsertHook);
}

test "ComponentInfo.of withCustomName" {
    const TestComponent = struct {
        pub const sage_name = "CustomName";
        data: u32,
    };

    const info = ComponentInfo.of(TestComponent);
    try std.testing.expectEqualStrings("CustomName", info.name);
    try std.testing.expectEqual(4, info.size);
    try std.testing.expectEqual(Alignment.@"4", info.alignment);
    try std.testing.expectEqual(null, info.deinit);
    try std.testing.expectEqual(null, info.onRemoveHook);
    try std.testing.expectEqual(null, info.onInsertHook);
}

test "ComponentInfo.of customDeinit" {
    const TestComponent = struct {
        deinit_count: *u32,

        pub fn sageDeinit(self: *@This(), engine: *Engine) void {
            _ = engine;
            self.deinit_count.* += 1;
        }
    };

    const info = ComponentInfo.of(TestComponent);
    try std.testing.expectEqual(@sizeOf(TestComponent), info.size);
    try std.testing.expectEqual(Alignment.of(TestComponent), info.alignment);
    try std.testing.expectEqual(null, info.onRemoveHook);
    try std.testing.expectEqual(null, info.onInsertHook);

    const fake_engine: *Engine = @ptrFromInt(@alignOf(Engine));
    var deinit_count: u32 = 0;
    var val = TestComponent{ .deinit_count = &deinit_count };
    info.deinit.?(&val, fake_engine);

    try std.testing.expectEqual(1, deinit_count);
}

/// The ID of a component.
///
/// Component IDs are runtime-generated and unique for each component
/// type. They should not be manually assigned or serialized to disk
/// because they are not meant to be stable across different runs of the
/// game (though they might often be).
pub const ComponentId = usize;

/// The list of components that have been registered so far.
///
/// `ComponentId`s are indices into this array.
components: ArrayListUnmanaged(ComponentInfo),
/// Maps UUIDs to component IDs.
uuids: Uuid.MapUnmanaged(ComponentId),

/// Registers a Zig type as a component.
///
/// # Known declarations
///
/// This function will look for different declarations in the provided
/// type.
///
/// ## `sage_uuid`
///
/// ```zig
/// const sage_uuid: Uuid = Uuid.parse("00000000-0000-0000-0000-000000000000");
/// ```
///
/// Components can be associated to a UUID which will can be used to identify
/// it across runs of the program. Unlike `ComponentId`s, UUIDs are stable
/// across different runs of the game.
///
/// This is the only required declaration.
///
/// The engine relies on UUIDs being unique. Giving the same UUID to two
/// different components will result in undefined behavior.
///
/// ## `sage_name`
///
/// ```zig
/// const sage_name: []const u8 = "custom name";
/// ```
///
/// A custom name to be used for debugging purposes. When not set, Sage
/// will use the name of the type (`@typeName(T)`).
///
/// ## `sageDeinit`
///
/// ```zig
/// fn sageDeinit(self: *Self, engine: *Engine) void {
///     // Perform any cleanup here
/// }
/// ```
///
/// A function to be called to destroy the component and release any resources
/// it may have allocated.
///
/// ## `sageOnInsertHook`
///
/// ```zig
/// fn sageOnInsertHook(self: *Self, engine: *Engine, entity: Entity) void {
///     // Perform any initialization here
/// }
/// ```
///
/// A function to be called when the component is inserted into an entity. The function
/// is called *after* the component has been inserted.
///
/// This can be used to keep invariants or perform bookkeeping.
///
/// ## `sageOnRemoveHook`
///
/// ```zig
/// fn sageOnRemoveHook(self: *Self, engine: *Engine, entity: Entity) void {
///     // Perform any cleanup here
/// }
/// ```
///
/// A function to be called when the component is removed from an entity. The function
/// is called *before* the component is removed.
///
/// This can be used to keep invariants or perform bookkeeping.
pub fn registerZigType(
    self: *Self,
    a: Allocator,
    comptime T: type,
) Allocator.Error!ComponentId {
    if (!@hasDecl(T, "sage_uuid")) {
        @compileError(
            \\
            \\Registering a Zig component requires a `sage_uuid` declaration:
            \\
            \\    const sage_uuid: sage.Uuid = sage.Uuid.parse("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx") catch unreachable;
            \\
        );
    }

    const uuid: Uuid = T.sage_uuid;
    const entry = try self.uuids.getOrPut(a, uuid);
    errdefer self.uuids.removeByPtr(entry.key_ptr);
    if (entry.found_existing)
        return entry.value_ptr.*;
    entry.value_ptr.* = self.components.items.len;
    try self.components.append(a, ComponentInfo.of(T));
}
