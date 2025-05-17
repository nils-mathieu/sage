//! Contains the complete state of an application
//! running using the Sage game engine.
//!
//! It is responsible for managing the application's
//! lifecycle, including initialization, update, and
//! state management.

const std = @import("std");
const Engine = @This();
const fxhash = @import("utility/fxhash.zig");
const oom = @import("utility/errors.zig").oom;
const Allocator = std.mem.Allocator;
const testing_components = @import("utility/testing_components.zig");

pub const EntityAllocator = @import("Engine/EntityAllocator.zig");
pub const ComponentRegistry = @import("Engine/ComponentRegistry.zig");
pub const Table = @import("Engine/Table.zig");

const ComponentId = ComponentRegistry.ComponentId;
const EntitySlotIndex = EntityAllocator.EntitySlotIndex;
const Entity = EntityAllocator.Entity;

// ============================================================================
// Archetypes
// ============================================================================

/// A set of components.
///
/// # Invariants
///
/// The slice representing an archetype must:
///
/// - Contain no duplicates.
///
/// - Be sorted in ascending order.
///
/// This ensures that the result of hashing the archetype is always consistent.
///
/// # Alignment
///
/// An `Archetype` slice is aligned with the same requirement as the `u64` type,
/// allowing us to hash it efficiently.
pub const Archetype = []align(@alignOf(u64)) const ComponentId;

/// The hash-map context used to hash `Archetype` slices.
const ArchetypeHashMapContext = struct {
    pub fn hash(self: @This(), key: Archetype) u64 {
        _ = self;
        return fxhash.hashAligned(std.mem.sliceAsBytes(key));
    }

    pub fn eql(self: @This(), lhs: Archetype, rhs: Archetype) bool {
        _ = self;
        return std.mem.eql(ComponentId, lhs, rhs);
    }
};

/// A map of `Archetype` to some value of type `V`.
pub fn ArchetypeHashMapUnmanaged(comptime V: type, comptime max_load_percentage: u64) type {
    return std.HashMapUnmanaged(Archetype, V, ArchetypeHashMapContext, max_load_percentage);
}

/// Duplicates an archetype.
pub fn dupeArchetype(allocator: Allocator, archetype: Archetype) Allocator.Error!Archetype {
    const new_archetype = try allocator.alignedAlloc(ComponentId, std.mem.Alignment.of(u64), archetype.len);
    @memcpy(new_archetype, archetype);
    return new_archetype;
}

/// The index of a table responsible for storing the components
/// of an entity.
pub const TableIndex = usize;

/// The row of an entity within a table.
pub const TableRow = usize;

/// The location of an entity.
///
/// Unlike the `Entity` ID object, this is *not* stable across
/// insertion and deletion of components and insertion of new
/// entities.
pub const EntityLocation = struct {
    /// The index of the table that stores the entity's components.
    table_index: TableIndex,
    /// The row of the entity within its table.
    table_row: TableRow,
};

// ============================================================================
// Fields
// ============================================================================

/// The allocator that the engine uses
/// to allocate memory.
///
/// The complete application should generally be using this
/// allocator when it needs to allocate memory.
///
/// Consequently, this allocator implementation should
/// be support freeing memory and granular allocations.
allocator: Allocator,

/// The entity allocator that is responsible for creating new
/// `Entity` IDs.
entity_allocator: EntityAllocator = .{},

/// The component registry that is responsible for storing
/// information about the components that the engine
/// supports.
component_registry: ComponentRegistry = .{},

/// A mapping from an entity's `Archetype` to the index of the table that is
/// responsible for storing its components.
archetypes: ArchetypeHashMapUnmanaged(TableIndex, std.hash_map.default_max_load_percentage) = .empty,

/// The list of all tables responsible for storing entity components.
///
/// To determine in which table a particular entity's components are
/// stored, there is two ways:
///
/// 1. The `archetypes` map can be used to map an entity's `Archetype` to the index of
///    the table that stores it.
///
/// 2. The `entity_allocator` stores along with each entity its location within the
///    table.
///
/// In other words, if you have the set of components (i.e. the `Archetype`), you can
/// get the table that contains the entity by looking into the `archetypes` map.
///
/// If you have the entity ID, you can get the table that contains the entity by looking
/// into the `entity_allocator`.
tables: std.ArrayListUnmanaged(Table) = .empty,

// ============================================================================
// Lifecycle
// ============================================================================

/// Initializes a new `Engine` instance.
///
/// The created engine must be destroyed using the
/// `deinit` function once it is no longer in use.
pub fn init(allocator: Allocator) Engine {
    return Engine{ .allocator = allocator };
}

/// Releases the resources owned by the engine.
///
/// After this function has been called, the engine
/// instance must no longer be used.
pub fn deinit(self: *Engine) void {
    for (self.tables.items) |*table| table.deinit(self.component_registry, self.allocator);
    self.tables.deinit(self.allocator);
    var archetype_iter = self.archetypes.keyIterator();
    while (archetype_iter.next()) |archetype| self.allocator.free(archetype.*);
    self.archetypes.deinit(self.allocator);
    self.component_registry.deinit(self.allocator);
    self.entity_allocator.deinit(self.allocator);
    self.* = undefined;
}

// ============================================================================
// Entity Management
// ============================================================================

/// A reference to an entity.
pub const EntityRef = struct {
    /// The engine that created this entity.
    engine: *Engine,
    /// The slot index of the entity.
    entity: Entity,

    /// Despawns the entity from the engine.
    pub fn despawn(self: EntityRef) void {
        const location = self.getLocation();
        const table = self.getTable();

        self.engine.entity_allocator.deallocate(self.engine.allocator, self.entity.slot_index);
        table.swapRemoveDeinit(self.engine.allocator, self.engine.component_registry, location.table_row);

        // The entity was the last entity in the table. We don't need to
        // fix any moved entities because locations remained stable.
        if (location.table_row == table.len) return;

        const moved_entity_slot_index = table.entities[location.table_row];
        self.engine.entity_allocator.getMetadataPtr(moved_entity_slot_index).* = location;
    }

    /// Returns the location of the entity.
    pub inline fn getLocation(self: EntityRef) EntityLocation {
        return self.engine.entity_allocator.getMetadata(self.entity.slot_index);
    }

    /// Returns a table that contains entities with the same archetype
    /// as this entity.
    pub inline fn getTable(self: EntityRef) *Table {
        return &self.engine.tables.items[self.getLocation().table_index];
    }

    /// Returns a pointer to the component with the given ID.
    ///
    /// # Valid Usage
    ///
    /// The caller must ensure that the entity has the component.
    pub fn getComponentByIdAssumePresent(self: EntityRef, component_id: ComponentId) *anyopaque {
        return self.getTable().getComponentAssumePresent(
            self.engine.component_registry,
            component_id,
            self.getLocation().table_row,
        );
    }

    /// Returns a pointer to one of the components of the entity by its ID.
    ///
    /// If the entity does not have the provided component, this function returns
    /// `null`.
    pub fn getComponentById(self: EntityRef, component_id: ComponentId) ?*anyopaque {
        return self.getTable().getComponent(
            self.engine.component_registry,
            component_id,
            self.getLocation().table_row,
        );
    }

    /// Returns a pointer to one of the components of the entity by its
    /// type.
    ///
    /// # Valid Usage
    ///
    /// The caller must ensure that the entity has the component.
    pub fn getComponentAssumePresent(self: EntityRef, comptime T: type) *T {
        const component_id = self.engine.component_registry.getIdZig(T) orelse unreachable;
        return @ptrCast(@alignCast(self.getComponentByIdAssumePresent(component_id)));
    }

    /// Returns a pointer to one of the components of the entity
    /// by its type.
    pub fn getComponent(self: EntityRef, comptime T: type) ?*T {
        const component_id = self.engine.component_registry.getIdZig(T) orelse return null;
        return @ptrCast(@alignCast(self.getComponentById(component_id)));
    }
};

/// Returns the ID of the table that contains entities with the
/// given archetype.
///
/// If no mapping exists from the provided archetype, a new
/// one is created.
pub fn getTableIdForArchetype(self: *Engine, archetype: Archetype) TableIndex {
    const entry = self.archetypes.getOrPut(self.allocator, archetype) catch oom();
    if (entry.found_existing) return entry.value_ptr.*;
    entry.value_ptr.* = self.tables.items.len;
    self.tables.append(self.allocator, Table.initForArchetype(self.allocator, archetype)) catch oom();
    entry.key_ptr.* = dupeArchetype(self.allocator, archetype) catch oom();
    return entry.value_ptr.*;
}

/// Spawns an entity in the provided table.
///
/// # Valid Usage
///
/// This function does not initialize the created entity. It is the responsibility
/// of the caller to setup the entity's components.
pub fn spawnInTable(self: *Engine, table_index: TableIndex) EntityRef {
    const entity = self.entity_allocator.allocateOne(self.allocator);
    const table = &self.tables.items[table_index];
    self.entity_allocator.getMetadataPtr(entity.slot_index).* = EntityLocation{
        .table_index = table_index,
        .table_row = table.len,
    };
    table.ensureUnusedCapacity(self.component_registry, self.allocator, 1);
    table.addOneAssumeCapacity(entity.slot_index);
    return EntityRef{
        .engine = self,
        .entity = entity,
    };
}

/// Spawns an entity of the provided archetype.
///
/// # Valid Usage
///
/// This function does not initialize the created entity. It is the responsibility
/// of the caller to setup the entity's components.
pub fn spawnInArchetype(self: *Engine, archetype: Archetype) EntityRef {
    const table_id = self.getTableIdForArchetype(archetype);
    return self.spawnInTable(table_id);
}

/// Spawns a new entity.
///
/// The provided `bundle` is used to initialize the entity's components. Each
/// field of the bundle corresponds to a component type to initialize.
pub fn spawn(self: *Engine, bundle: anytype) EntityRef {
    // Get information about the bundle type.

    const Bundle = @TypeOf(bundle);
    const bundle_info = @typeInfo(Bundle);
    if (bundle_info != .@"struct") {
        @compileError("Engine.spawn: The provided bundle type must be a struct");
    }
    const fields = bundle_info.@"struct".fields;

    var component_ids: [fields.len]ComponentId = undefined;
    inline for (fields, &component_ids) |field, *dst| {
        dst.* = self.component_registry.registerZig(self.allocator, field.type);
    }

    comptime {
        for (0..fields.len - 1) |i| {
            for (i + 1..fields.len) |j| {
                if (fields[i].type == fields[j].type) {
                    @compileError(std.fmt.comptimePrint(
                        \\Engine.spawn: The provided bundle type contains duplicate component
                        \\of type `{s}`
                    , .{ComponentRegistry.ComponentInfo.of(fields[i].type).debug_name}));
                }
            }
        }
    }

    // Create an `Archetype` for the bundle.

    var archetype: [fields.len]ComponentId align(@alignOf(u64)) = component_ids;

    const SortContext = struct {
        pub fn lessThan(self_ctx: @This(), lhs: ComponentId, rhs: ComponentId) bool {
            _ = self_ctx;
            return lhs < rhs;
        }
    };

    std.sort.heap(ComponentId, &archetype, SortContext{}, SortContext.lessThan);

    // Spawn the entity and initialize its components.

    const entity = self.spawnInArchetype(&archetype);
    const table = entity.getTable();
    const row = entity.getLocation().table_row;

    inline for (fields, &component_ids) |field, component_id| {
        const dst: *field.type = @ptrCast(@alignCast(table.getComponentAssumePresent(self.component_registry, component_id, row)));
        dst.* = @field(bundle, field.name);
    }

    return entity;
}

/// Returns whether the provided `Entity` ID is alive in the engine or not.
pub fn isAlive(self: Engine, entity: Entity) bool {
    return self.entity_allocator.contains(entity);
}

test "spawn" {
    var engine = Engine.init(std.testing.allocator);
    defer engine.deinit();

    const entity = engine.spawn(.{
        testing_components.ComponentA{ .data = 123 },
        testing_components.ComponentC.fromSlice("hello"),
    });

    try std.testing.expectEqual(123, entity.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("hello", entity.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expect(engine.isAlive(entity.entity));
}

test "despawn" {
    var engine = Engine.init(std.testing.allocator);
    defer engine.deinit();

    const entity = engine.spawn(.{
        testing_components.ComponentA{ .data = 123 },
        testing_components.ComponentC.fromSlice("test"),
    });

    try std.testing.expectEqual(123, entity.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test", entity.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expect(engine.isAlive(entity.entity));
    entity.despawn();
    try std.testing.expect(!engine.isAlive(entity.entity));
}

test "despawn middle of table" {
    var engine = Engine.init(std.testing.allocator);
    defer engine.deinit();

    const e1 = engine.spawn(.{
        testing_components.ComponentA{ .data = 1 },
        testing_components.ComponentC.fromSlice("test1"),
    });
    const e2 = engine.spawn(.{
        testing_components.ComponentA{ .data = 2 },
        testing_components.ComponentC.fromSlice("test2"),
    });
    const e3 = engine.spawn(.{
        testing_components.ComponentA{ .data = 3 },
        testing_components.ComponentC.fromSlice("test3"),
    });
    const e4 = engine.spawn(.{
        testing_components.ComponentA{ .data = 4 },
        testing_components.ComponentC.fromSlice("test4"),
    });

    try std.testing.expectEqual(1, e1.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test1", e1.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expectEqual(2, e2.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test2", e2.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expectEqual(3, e3.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test3", e3.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expectEqual(4, e4.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test4", e4.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expect(engine.isAlive(e1.entity));
    try std.testing.expect(engine.isAlive(e2.entity));
    try std.testing.expect(engine.isAlive(e3.entity));
    try std.testing.expect(engine.isAlive(e4.entity));

    e2.despawn();

    try std.testing.expectEqual(1, e1.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test1", e1.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expectEqual(3, e3.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test3", e3.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expectEqual(4, e4.getComponent(testing_components.ComponentA).?.data);
    try std.testing.expectEqualStrings("test4", e4.getComponent(testing_components.ComponentC).?.data.items);
    try std.testing.expect(engine.isAlive(e1.entity));
    try std.testing.expect(!engine.isAlive(e2.entity));
    try std.testing.expect(engine.isAlive(e3.entity));
    try std.testing.expect(engine.isAlive(e4.entity));
}
