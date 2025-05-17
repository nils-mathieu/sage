//! A "structore-of-arrays" data structure for storing the components
//! of a collection of entities.
//!
//! All the entities in a given `Table` share the same set of components.
//! Each component is given an array which contains the value of
//! that component for each entity in the table.

const ComponentRegistry = @import("ComponentRegistry.zig");
const ComponentId = ComponentRegistry.ComponentId;
const SparseSet = @import("../utility/SparseSet.zig").SparseSet;
const EntityAllocator = @import("EntityAllocator.zig");
const EntitySlotIndex = EntityAllocator.EntitySlotIndex;
const Table = @This();
const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;
const Archetype = @import("../Engine.zig").Archetype;

/// Represents a column in a table.
pub const Column = struct {
    /// The ID of the component that is stored in this column.
    ///
    /// This is the reverse of going through the `columns.get(component_id)`
    /// field.
    component_id: ComponentId,

    /// The data pointer for the column.
    ///
    /// The length and capacity associated with this pointer
    /// are stored in the parent `Table` object.
    data: [*]u8 = @ptrFromInt(@alignOf(u8)),
};

/// The columns of the table.
columns: SparseSet(Column, u8) = .{},

/// The entities that are stored in this table.
///
/// Only the slot index of each entity is stored here, instead
/// of the whole `Entity` object, because we know that all
/// entities here are alive and valid, meaning that we can
/// always retrieve the associated generation number.
entities: [*]EntitySlotIndex = @ptrFromInt(@alignOf(EntitySlotIndex)),

/// The number of entities stored in this table.
///
/// This is the number of objects referenced in the `entities` array, as
/// well as in each column of the table.
len: usize = 0,

/// The capacity of the table.
///
/// This is the number of entities that can be stored in the table before
/// it needs to be resized.
cap: usize = 0,

/// Initializes a new `Table` for the provided archetype.
pub fn initForArchetype(allocator: Allocator, archetype: Archetype) Table {
    var table = Table{};
    if (archetype.len > 0) {
        table.columns.ensureCapacityForKey(allocator, archetype[archetype.len - 1]) catch oom();
        table.columns.ensureUnusedValueCapacity(allocator, archetype.len) catch oom();
        for (archetype) |component_id| {
            table.columns.putNoClobberAssumeCapacity(
                component_id,
                Column{ .component_id = component_id },
            );
        }
    }
    return table;
}

/// Releases the resources that the table is using.
pub fn deinit(self: *Table, reg: ComponentRegistry, allocator: Allocator) void {
    allocator.free(self.entities[0..self.cap]);

    for (self.columns.values()) |*column| {
        const info = reg.get(column.component_id);
        if (info.deinitFn) |deinitComponentFn| {
            for (0..self.len) |i| {
                deinitComponentFn(column.data + i * info.size, allocator);
            }
        }
        allocator.rawFree(
            column.data[0 .. self.cap * info.size],
            info.alignment,
            @returnAddress(),
        );
    }

    self.columns.deinit(allocator);
}

/// Ensures that the table has the required capacity to store a total of `new_cap` entities
/// without reallocating.
pub fn ensureTotalCapacity(self: *Table, reg: ComponentRegistry, allocator: Allocator, new_cap: usize) void {
    if (new_cap <= self.cap) return;

    self.entities = (allocator.realloc(self.entities[0..self.cap], new_cap) catch oom()).ptr;

    for (self.columns.values()) |*column| {
        const info = reg.get(column.component_id);

        column.data = (rawRealloc(
            allocator,
            column.data[0 .. self.cap * info.size],
            info.alignment,
            new_cap * info.size,
            self.len * info.size,
            @returnAddress(),
        ) catch oom());
    }

    self.cap = new_cap;
}

/// Makes sure that the table has the required capacity to store an additional of `additional`
/// entities without reallocating.
pub fn ensureUnusedCapacity(self: *Table, reg: ComponentRegistry, allocator: Allocator, additional: usize) void {
    const requested = std.math.add(usize, self.len, additional) catch oom();
    if (requested > self.cap) {
        self.ensureTotalCapacity(reg, allocator, @max(self.cap *| 2, requested));
    }
}

/// Adds an entity to the table.
///
/// # Valid Usage
///
/// The caller must ensure that the table has enough capacity to store
/// the new entity.
///
/// The caller is responsible for initializing the components of the entity.
/// Specifically, the caller must ensure that all components of the entity
/// are initialized before using the table in any other ways.
pub fn addOneAssumeCapacity(self: *Table, entity: EntitySlotIndex) void {
    std.debug.assert(self.len < self.cap);
    self.entities[self.len] = entity;
    self.len += 1;
}

/// Removes the entity at the given index.
///
/// # Valid Usage
///
/// This function assumes taht the provided index is valid.
pub fn swapRemoveDeinit(self: *Table, allocator: Allocator, reg: ComponentRegistry, index: usize) void {
    std.debug.assert(index < self.len);

    // Invoke the destructor of the entity's components.
    for (self.columns.values()) |*column| {
        const info = reg.get(column.component_id);
        if (info.deinitFn) |deinitComponentFn| {
            deinitComponentFn(column.data + info.size * index, allocator);
        }
    }

    self.len -= 1;

    // If we just removed the last element, we don't have anything more to do.
    if (self.len == index) return;

    // Otherwise, move the last element of the table to the removed index to
    // fill to created hole.
    self.entities[index] = self.entities[self.len];
    for (self.columns.values()) |*column| {
        const info = reg.get(column.component_id);
        const dst = column.data + info.size * index;
        const src = column.data + info.size * self.len;
        @memcpy(dst[0..info.size], src[0..info.size]);
    }
}

/// Returns a pointer to the component of the entity at the given index.
///
/// # Valid Usage
///
/// This function assumes that:
///
/// - The provided `index` reffers to a valid entity that is in bounds of the table.
///
/// - The component must be present in the table.
pub fn getComponentAssumePresent(self: Table, reg: ComponentRegistry, component_id: ComponentId, index: usize) *anyopaque {
    std.debug.assert(index < self.len);
    const info = reg.get(component_id);
    return self.columns.getAssumePresent(component_id).data + info.size * index;
}

/// Returns a pointer to the component of the entity at the given index.
///
/// # Valid Usage
///
/// This function assumes that the provided `index` refers to a valid entity that is in bounds of the table.
pub fn getComponent(self: Table, reg: ComponentRegistry, component_id: ComponentId, index: usize) ?*anyopaque {
    std.debug.assert(index < self.len);
    const info = reg.get(component_id);
    const column = self.columns.get(component_id) orelse return null;
    return column.data + info.size * index;
}

fn rawRealloc(
    allocator: Allocator,
    old_mem: []u8,
    alignment: Alignment,
    new_n: usize,
    copy_len: usize,
    return_address: usize,
) Allocator.Error![*]u8 {
    if (old_mem.len == 0) {
        return allocator.rawAlloc(new_n, alignment, return_address) orelse return error.OutOfMemory;
    }
    if (new_n == 0) {
        allocator.rawFree(old_mem, alignment, return_address);
        return @ptrFromInt(alignment.toByteUnits());
    }
    if (allocator.rawRemap(old_mem, alignment, new_n, return_address)) |new_mem| {
        return new_mem;
    } else {
        const new_mem = allocator.rawAlloc(new_n, alignment, return_address) orelse return error.OutOfMemory;
        @memcpy(new_mem[0..copy_len], old_mem[0..copy_len]);
        return new_mem;
    }
}

fn oom() noreturn {
    std.debug.panic("Table: out of memory", .{});
}
