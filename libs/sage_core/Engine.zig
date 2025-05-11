//! Contains the complete state of the game. One instance of this struct
//! should be created at the start of the game and passed around.

pub const ComponentRegistry = @import("Engine/ComponentRegistry.zig");
pub const EntityAllocator = @import("Engine/EntityAllocator.zig");
pub const Table = @import("Engine/Table.zig");

const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const ComponentId = ComponentRegistry.ComponentId;
const fold_hash = @import("utility/fold_hash.zig");
const Entity = EntityAllocator.Entity;

const Self = @This();

// =================================================================================================
// Archetypes
// =================================================================================================

/// A set of component IDs.
///
/// Slices of this type must be aligned like a `u128` element facilitate
/// hashing and comparison operations.
///
/// The slice must be sorted by increasing component ID and must not
/// contain duplicates.
pub const Archetype = []align(@alignOf(u128)) ComponentId;

const Archetypes = std.HashMapUnmanaged(
    Archetype,
    TableIndex,
    struct {
        pub fn hash(self: @This(), archetype: Archetype) u64 {
            _ = self;
            return fold_hash.computeHashAligned(std.mem.sliceAsBytes(archetype));
        }

        pub fn eql(self: @This(), a: Archetype, b: Archetype) bool {
            _ = self;
            return std.mem.eql(ComponentId, a, b);
        }
    },
    std.hash_map.default_max_load_percentage,
);

/// Duplicates the provided archetype.
fn dupeArchetype(allocator: Allocator, archetype: Archetype) Allocator.Error!Archetype {
    const ret = try allocator.alignedAlloc(ComponentId, std.mem.Alignment.of(u128), archetype.len);
    @memcpy(ret, archetype);
    return ret;
}

// =================================================================================================
// Entity Location
// =================================================================================================

/// The index of a table. This corresponds to a value in the `tables` array.
pub const TableIndex = usize;

/// The index of a entity in a table.
pub const TableRow = usize;

/// The location of an entity.
pub const EntityLocation = struct {
    /// The index of the table that contains the entity.
    table_index: TableIndex,
    /// The index of the row in the table that contains the entity.
    table_row: TableRow,
};

// =================================================================================================
// Fields
// =================================================================================================

/// The general purpose allocator used throughout the game.
///
/// This allocator must support general allocations and free operations
/// to make sure that memory can be granularly managed.
allocator: Allocator,

/// Contains information about the types (or components) that the engine can store
/// and manage.
component_registry: ComponentRegistry,

/// The entity allocator is responsible for managing the creation and
/// removal of `Entity` IDs across the engine.
///
/// It allows for reserving IDs concurrently, ensuring that the engine
/// can handle object creation over multiple threads without issues.
entity_allocator: EntityAllocator,

/// The list of entity tables.
///
/// Each table is responsible for storing the components of a specific kind of entity.
tables: ArrayListUnmanaged(Table),

/// Converts `Archetype`s to the table index that contains entities
/// with that archetype.
archetypes: Archetypes,

/// Releases the resources that were allocated for the engine.
pub fn deinit(self: *Self) void {
    for (self.tables.items) |*table| {
        table.deinit(self.allocator, self.component_registry);
    }
    self.tables.deinit(self.allocator);

    var archetypes = self.archetypes.keyIterator();
    while (archetypes.next()) |key| self.allocator.free(key.*);
    self.archetypes.deinit(self.allocator);

    self.component_registry.deinit(self.allocator);
    self.entity_allocator.deinit(self.allocator);
}

// =================================================================================================
// Entity Management
// =================================================================================================

/// Flushes pending operations on the engine.
///
/// # Thread Safety
///
/// The caller must make sure that the function has exclusive access to the engine
/// in its entirety while it executes.
pub fn flush(self: *Self) Allocator.Error!void {
    if (self.entity_allocator.reservedEntities() > 0) {
        @branchHint(.cold);
        try self.flushReservedEntities();
    }
}

fn flushReservedEntities(self: *Self) Allocator.Error!void {
    const table_index = try self.getTableIndexForArchetype(&.{});
    const table = &self.tables.items[table_index];
    try table.ensureUnusedCapacity(
        self.allocator,
        self.component_registry,
        self.entity_allocator.reservedEntities(),
    );

    var flushed = try self.entity_allocator.flushReservedEntities(self.allocator);
    while (flushed.next()) |entity| {
        const table_row = table.len;
        table.addOneAssumeCapacity();
        table.entities[table_row] = entity.slot;
        self.entity_allocator.getMetadataPtr(entity.slot).* = EntityLocation{
            .table_index = table_index,
            .table_row = table_row,
        };
    }
}

/// Returns the table index of the table that contains the entities following the provided
/// archetype.
///
/// # Valid Usage
///
/// The caller must make sure that the provided archetype is valid. Specifically, it must contain
/// component IDs that are unique and sorted in increasing order.
pub fn getTableIndexForArchetype(self: *Self, archetype: Archetype) Allocator.Error!TableIndex {
    const entry = try self.archetypes.getOrPut(self.allocator, archetype);
    if (entry.found_existing) return entry.value_ptr.*;
    errdefer self.archetypes.removeByPtr(entry.key_ptr);

    // Duplicate the archetype to make sure we can use it as long as we want.
    entry.key_ptr.* = try dupeArchetype(self.allocator, archetype);
    errdefer self.allocator.free(entry.key_ptr.*);

    const table_index = self.tables.items.len;
    try self.tables.append(
        self.allocator,
        try Table.init(self.allocator, self.component_registry, archetype),
    );
    return table_index;
}

/// Spawns a new entity in the engine.
///
/// # Valid Usage
///
/// The caller is responsible for making sure that:
///
/// - The provided table index is valid.
///
/// - The entity is initialized before anything is done with it (including deleting it).
///
/// # Component Hooks
///
/// This method will not invoke eventual component hooks that might need to run. It is the
/// responsibility of the caller to make sure to call them if needed once the components of the
/// entity have been initialized.
pub fn spawnInTable(self: *Self, table_index: TableIndex) Allocator.Error!EntityRef {
    const table = &self.tables.items[table_index];
    try table.ensureUnusedCapacity(self.allocator, self.component_registry, 1);
    const entity = try self.entity_allocator.allocate(self.allocator);
    const table_row = table.len;
    table.addOneAssumeCapacity();
    table.entities[table_row] = entity.slot;
    const location = EntityLocation{
        .table_index = table_index,
        .table_row = table_row,
    };
    self.entity_allocator.getMetadataPtr(entity.slot).* = location;
    return EntityRef{
        .parent = self,
        .location = location,
        .entity = entity,
    };
}

/// Spawns a new entity in the engine.
///
/// # Valid Usage
///
/// The caller is responsible for making sure that:
///
/// - The provided archetype is valid. Specifically, it must contain component IDs that are
///   unique and sorted in increasing order.
///
/// - The entity is initialized before anything is done with it (including deleting it).
///
/// # Component Hooks
///
/// This method will not invoke eventual component hooks that might need to run. It is the
/// responsibility of the caller to make sure to call them if needed once the components of the
/// entity have been initialized.
pub fn spawnInArchetype(self: *Self, archetype: Archetype) Allocator.Error!EntityRef {
    const table_index = try self.getTableIndexForArchetype(archetype);
    return self.spawnInTable(table_index);
}

/// Spawns a new entity in the engine.
///
/// # Input
///
/// The `bundle` parameter must be a struct that contains the components to be
/// inserted in the entity.
pub fn spawn(self: *Self, bundle: anytype) Allocator.Error!EntityRef {
    //
    // Gather information about the components that are going to be used.
    //
    const Bundle = @TypeOf(bundle);
    const bundle_type_info = @typeInfo(Bundle);

    if (bundle_type_info != .@"struct") {
        @compileError(
            \\
            \\Engine.spawn: The provided bundle type is not a struct.
            \\
        );
    }

    const fields = bundle_type_info.@"struct".fields;

    //
    // Get the component IDs of the components to insert in the entity.
    //

    var component_ids: [fields.len]ComponentId = undefined;

    for (fields, &component_ids) |field, *component_id| {
        component_id.* = try self.component_registry.registerZigType(self.allocator, field.type);
    }

    //
    // Build the archetype for the created entity.
    //

    var archetype: [fields.len]ComponentId align(@alignOf(u128)) = undefined;
    @memcpy(archetype, component_ids);

    const SortContext = struct {
        pub fn lessThan(_self: @This(), a: ComponentId, b: ComponentId) bool {
            _ = _self;
            return a < b;
        }
    };

    // Sort the archetype to make sure it is valid.
    std.sort.heapContext(ComponentId, &archetype, SortContext{});

    // Determine whether the archetype contains any duplicate component IDs.
    for (archetype, archetype[1..]) |component_id, next_component_id| {
        if (component_id == next_component_id) {
            @compileError(
                \\
                \\Engine.spawn: The provided bundle contains duplicate components.
                \\
            );
        }
    }

    //
    // Spawn the entity.
    //

    const entity_ref = try self.spawnInArchetype(&archetype);
    const table = &self.tables.items[entity_ref.location.table_index];

    //
    // Initialize the components of the entity.
    //

    for (fields, &component_ids) |field, component_id| {
        const column = table.columns.getAssumePresent(component_id);
        const dst: *field.type = @alignCast(@ptrCast(column.data + entity_ref.location.table_row * @sizeOf(field.type)));
        dst.* = @field(bundle, field.name);
    }

    //
    // Invoke component hooks.
    //

    for (table.columns.values()) |column| {
        const info = self.component_registry.get(column.component_id);
        if (info.onInsertHook) |onInsertHook| {
            onInsertHook(
                column.data + entity_ref.location.table_row * info.size,
                self,
                entity_ref.entity,
            );
        }
    }

    return entity_ref;
}

/// A reference to a live entity.
///
/// # Remarks
///
/// An `EntityRef` is a reference to an entity that is currently alive in
/// the engine. Inserting new entities or moving components around while
/// an `EntityRef` is alive may invalidate it.
pub const EntityRef = struct {
    /// The parent engine that contains the entity.
    parent: *Self,
    /// The location of the entity in the engine.
    location: EntityLocation,
    /// The slot index of the entity.
    entity: Entity,

    /// Despawns the entity from the engine.
    ///
    /// # Valid Usage
    ///
    /// The caller must no longer use the entity after this method is called.
    pub fn despawn(self: *EntityRef) Allocator.Error!void {
        const table = &self.parent.tables.items[self.location.table_index];

        //
        // Invoke component hooks.
        //

        for (table.columns.values()) |column| {
            const info = self.parent.component_registry.get(column.component_id);
            if (info.onRemoveHook) |onRemoveHook| {
                onRemoveHook(
                    column.data + self.location.table_row * info.size,
                    self.parent,
                    self.entity,
                );
            }
        }

        //
        // Remove the entity from the table.
        //

        table.swapRemove(self.parent.component_registry, self.location.table_row);

        if (self.location.table_row != table.len) {
            // An entity has moved to take the place of the removed entity.
            const moved_entity = table.entities[self.location.table_row];
            self.parent.entity_allocator.getMetadataPtr(moved_entity).* = self.location;
        }

        //
        // We're done!
        //

        self.* = undefined;
    }
};

/// Returns an `EntityRef` that points to the entity with the provided ID.
///
/// # Valid Usage
///
/// The caller must make sure that the slot index of the entity is valid.
pub fn getEntityAssumeValid(self: *Self, entity: Entity) EntityRef {
    const location = self.entity_allocator.getMetadata(entity.slot);
    return EntityRef{ .parent = self, .location = location, .entity = entity };
}

/// Returns an `EntityRef` that points to the entity with the provided ID.
pub fn getEntity(self: *Self, entity: Entity) ?EntityRef {
    if (self.entity_allocator.containsAssumeNoPending(entity)) {
        return self.getEntityAssumeValid(entity);
    } else {
        return null;
    }
}
