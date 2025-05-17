//! The component registry is responsible for managing the registration
//! of component types within the engine.
//!
//! A component is a piece of data that the engine can manage and
//! update over time. Components are usually attached to an entity
//! and can be accessed and modified by systems.
//!
//! The easiest way to create a component is to use a Zig type with
//! special declarations. The specifics can be found bellow.
//!
//! # Zig Type as components
//!
//! It's possible to use an existing Zig type as a component. The
//! following describes how to make it work, and what can be configured
//! for it.
//!
//! ## UUID
//!
//! The only required declaration for a Zig component is the `component_uuid`
//! declaration. It must have the type of `Uuid` and be globally unique.
//!
//! You should generally generate a random UUIDv4 and copy it to your
//! source code. Collisions are practically impossible.
//!
//! ```zig
//! pub const component_uuid = Uuid.comptimeParse("a94e7ff1-7c90-42d5-9938-2d94bc7a2fa8");
//! ```
//!
//! ## Deinitialization
//!
//! When a component is destroyed, a deinitialization sequence can be invoked
//! by the engine.
//!
//! For this, components can have a `deinit` method with the following signature:
//!
//! ```zig
//! pub fn deinit(self: *@This()) void;
//!
//! // or
//!
//! pub fn deinit(self: *@This(), allocator: Allocator) void;
//! ```
//!
//! The allocator passed to the second version is the one that the `Engine` object
//! uses to allocate memory.
//!
//! ## Debug name
//!
//! It is possible to specify a custom debug name for the component. This name will
//! only be used to debug the component.
//!
//! When not specified, the type's name will be used.
//!
//! ```zig
//! pub const component_debug_name = "MyComponent";
//! ```

const std = @import("std");
const Alignment = std.mem.Alignment;
const Allocator = std.mem.Allocator;
const Uuid = @import("../Uuid.zig");
const ComponentRegistry = @This();
const oom = @import("../utility/errors.zig").oom;

/// Stores information about a registered component type.
pub const ComponentInfo = struct {
    /// The debug name of the component.
    debug_name: []const u8,

    /// The size in bytes of the component.
    size: usize,
    /// The alignment requirement of the component.
    alignment: Alignment,

    /// The deinitialization function of the component, if
    /// one has been registered.
    deinitFn: ?*const fn (self: *anyopaque, allocator: Allocator) void,

    /// Returns the `ComponentInfo` associated with the provided Zig type.
    ///
    /// The interface expected by this function is specified in the
    /// documentation of the `ComponentRegistry` struct.
    pub fn of(comptime T: type) ComponentInfo {
        const Fns = struct {
            pub fn deinit_1(component: *anyopaque, allocator: Allocator) void {
                _ = allocator;
                T.deinit(@ptrCast(@alignCast(component)));
            }

            pub fn deinit_2(component: *anyopaque, allocator: Allocator) void {
                T.deinit(@ptrCast(@alignCast(component)), allocator);
            }
        };

        const debug_name = a: {
            if (!@hasDecl(T, "component_debug_name")) {
                break :a @typeName(T);
            }

            break :a @as([]const u8, T.component_debug_name);
        };

        const deinitFn = a: {
            if (!@hasDecl(T, "deinit")) {
                break :a null;
            }

            const fn_info = @typeInfo(@TypeOf(T.deinit));
            if (fn_info != .@"fn")
                invalidDeinitSignature();

            const params = fn_info.@"fn".params;
            if (params.len != 1 and params.len != 2)
                invalidDeinitSignature();
            if (params[0].type.? != *T)
                invalidDeinitSignature();
            if (params.len > 1 and params[1].type.? != Allocator)
                invalidDeinitSignature();

            if (params.len == 1) {
                break :a Fns.deinit_1;
            } else if (params.len == 2) {
                break :a Fns.deinit_2;
            } else {
                unreachable;
            }
        };

        return .{
            .debug_name = debug_name,
            .size = @sizeOf(T),
            .alignment = Alignment.of(T),
            .deinitFn = deinitFn,
        };
    }

    fn invalidDeinitSignature() noreturn {
        @compileError(
            \\ComponentInfo.of:
            \\    The `.deinit` function must be a function with the
            \\    following signature:
            \\
            \\        fn (self: *@This(), allocator: Allocator) void;
            \\
        );
    }

    test "of nothing" {
        const TestType = struct {
            data: u32,
        };

        const info = ComponentInfo.of(TestType);
        try std.testing.expectStringEndsWith(info.debug_name, "TestType");
        try std.testing.expectEqual(Alignment.of(TestType), info.alignment);
        try std.testing.expectEqual(@sizeOf(TestType), info.size);
        try std.testing.expectEqual(null, info.deinitFn);
    }

    test "of with custom name" {
        const TestType = struct {
            pub const component_debug_name = "CustomName";

            data: u32,
        };

        const info = ComponentInfo.of(TestType);
        try std.testing.expectEqualStrings("CustomName", info.debug_name);
        try std.testing.expectEqual(Alignment.of(TestType), info.alignment);
        try std.testing.expectEqual(@sizeOf(TestType), info.size);
        try std.testing.expectEqual(null, info.deinitFn);
    }

    test "of with deinit" {
        var data: u32 = 0;

        const TestType1 = struct {
            data: *u32,

            pub fn deinit(self: *@This()) void {
                self.data.* = 42;
            }
        };

        const info1 = ComponentInfo.of(TestType1);

        var val1 = TestType1{ .data = &data };
        info1.deinitFn.?(@ptrCast(&val1), std.testing.allocator);
        try std.testing.expectEqual(42, val1.data.*);

        const TestType2 = struct {
            data: *u32,

            pub fn deinit(self: *@This(), allocator: Allocator) void {
                _ = allocator;
                self.data.* = 52;
            }
        };

        const info2 = ComponentInfo.of(TestType2);

        var val2 = TestType2{ .data = &data };
        info2.deinitFn.?(@ptrCast(&val2), std.testing.allocator);
        try std.testing.expectEqual(52, val2.data.*);
    }

    /// Returns the UUId of the provided type by reading the
    /// `component_uuid` declaration.
    ///
    /// See the documentation for `ComponentRegistry` for more information.
    pub fn uuidOf(comptime T: type) Uuid {
        if (!@hasDecl(T, "component_uuid")) {
            @compileError(std.fmt.comptimePrint(
                "The type `{s}` does not have a `component_uuid` declaration",
                .{@typeName(T)},
            ));
        }

        return @as(Uuid, T.component_uuid);
    }
};

/// The ID of a component registered with the engine.
pub const ComponentId = u32;

/// The list of components that have been registered
/// with the engine.
///
/// This list is indexed by `ComponentId`s.
components: std.ArrayListUnmanaged(ComponentInfo) = .empty,

/// The map of UUIDs to component IDs.
uuids: Uuid.HashMapUnmanaged(ComponentId, std.hash_map.default_max_load_percentage) = .empty,

/// Releases the resources that have been allocated by the
/// component registry.
pub fn deinit(self: *ComponentRegistry, allocator: Allocator) void {
    self.components.deinit(allocator);
    self.uuids.deinit(allocator);
}

/// Registers an anonymous component type with the engine.
///
/// Anonymous components do not have an associated UUID. This is only useful
/// when you need a local component to be present, and do not wish to make it
/// available to other modules.
pub fn registerAnonymous(self: *ComponentRegistry, allocator: Allocator, info: ComponentInfo) ComponentId {
    const id = std.math.cast(ComponentId, self.components.items.len) orelse tooManyComponents();
    self.components.append(allocator, info) catch oom();
    return id;
}

/// Registers a component type with the engine.
///
/// The provided UUID will be made available for other modules. They will
/// be able to refer to the component by its UUID.
///
/// # Valid Usage
///
/// This function assumes that the component has not been registered before.
/// Providing an existing UUID will result in an error when runtime safety
/// is enabled.
pub fn register(self: *ComponentRegistry, allocator: Allocator, uuid: Uuid, info: ComponentInfo) ComponentId {
    if (std.debug.runtime_safety) {
        if (self.uuids.get(uuid)) |id| {
            const other_component = &self.components.items[id];
            uuidCollision(uuid, other_component.debug_name, info.debug_name);
        }
    }

    const id = self.registerAnonymous(allocator, info);
    self.uuids.putNoClobber(allocator, uuid, id) catch oom();
    return id;
}

/// Registers a new component type with the engine using an existing
/// Zig type.
///
/// The Zig type must follow an interface that is detailed in the
/// documentation for the `ComponentRegistry` struct.
///
/// If the Zig type was already registered previously, the existing
/// component ID is returned.
pub fn registerZig(self: *ComponentRegistry, allocator: Allocator, comptime T: type) ComponentId {
    const key = ComponentInfo.uuidOf(T);
    const entry = self.uuids.getOrPut(allocator, key) catch oom();
    if (entry.found_existing) {
        // When runtime safety is enabled, quickly check if debug name matches. This
        // allows us to catch accidental UUID collisions.
        if (std.debug.runtime_safety) {
            const other_component = &self.components.items[entry.value_ptr.*];
            const new_info = ComponentInfo.of(T);
            if (!std.mem.eql(u8, other_component.debug_name, new_info.debug_name)) {
                uuidCollision(key, other_component.debug_name, new_info.debug_name);
            }
        }

        return entry.value_ptr.*;
    } else {
        entry.value_ptr.* = self.registerAnonymous(allocator, ComponentInfo.of(T));
        return entry.value_ptr.*;
    }
}

/// Returns the component ID associated with the given component type.
pub fn getIdZig(self: ComponentRegistry, comptime T: type) ?ComponentId {
    const uuid = ComponentInfo.uuidOf(T);
    return self.uuids.get(uuid);
}

/// Returns the `ComponentInfo` object associated with the given component ID.
///
/// # Valid Usage
///
/// This method assumes that the provided component ID is valid.
pub fn get(self: ComponentRegistry, component_id: ComponentId) ComponentInfo {
    return self.components.items[component_id];
}

fn uuidCollision(uuid: Uuid, already_here: []const u8, new_component: []const u8) noreturn {
    std.debug.panic(
        \\ComponentRegistry.register:
        \\
        \\Attempted to register component `{s}`
        \\with a UUID that was already used
        \\by component `{s}`.
        \\
        \\ UUID: {}
    , .{ already_here, new_component, uuid });
}

fn tooManyComponents() noreturn {
    std.debug.panic("ComponentRegistry: too many components registered", .{});
}
