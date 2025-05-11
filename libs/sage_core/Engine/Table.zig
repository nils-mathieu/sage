//! A "structure-of-arrays" (SoA) implementation to store a collection of
//! entities and their components.
//!
//! All entities in the table must have the same set of components.

const std = @import("std");
const Allocator = std.mem.Allocator;
const assert = std.debug.assert;

const ComponentRegistry = @import("ComponentRegistry.zig");
const ComponentId = ComponentRegistry.ComponentId;
const EntitySlotIndex = @import("EntityAllocator.zig").SlotIndex;
const SparseSet = @import("../utility/SparseSet.zig").SparseSet;
const Engine = @import("../Engine.zig");
const Archetype = Engine.Archetype;

const Self = @This();

/// A column in the table.
pub const Column = struct {
    /// The ID of the component stored in this column.
    component_id: ComponentId,
    /// The data pointer of the column.
    ///
    /// The length and capacity associated with this pointer are the same for
    /// all columns and are stored in the parent `Table` struct.
    data: [*]u8,
};

/// The number of initialized entities in the table.
len: usize = 0,
/// The number of entities that can possibility be stored in the table
/// without reallocating.
capacity: usize = 0,
/// The slot indices of the entities stored in the table.
entities: [*]EntitySlotIndex = @ptrFromInt(@alignOf(EntitySlotIndex)),
/// The columns of the table.
columns: SparseSet(Column, u8),

/// Creates a new `Table` isntance.
///
/// # Parameters
///
/// - `archetype`: The archetype of the entities that will be stored in the table. This is sorted
///   by increasing component ID. The input must not include twice the same component ID.
pub fn init(
    a: Allocator,
    reg: ComponentRegistry,
    archetype: Archetype,
) Allocator.Error!Self {
    var columns = SparseSet(Column, u8){};
    if (archetype.len != 0) {
        try columns.ensureSparseKey(a, archetype[archetype.len - 1]);
        try columns.ensureUnusedCapacity(a, archetype.len);
        for (archetype) |id| {
            columns.putAssumeCapacity(id, Column{
                .component_id = id,
                .data = @ptrFromInt(reg.components.items[id].alignment.toByteUnits()),
            });
        }
    }
    return Self{ .columns = columns };
}

/// Releases the memory used by the table.
///
/// # Remarks
///
/// This function will invoke the `deinit` method for all the components
/// stored in the table, but none of the eventual component hooks.
///
/// # Valid Usage
///
/// The caller must make sure that the table is not used anymore.
pub fn deinit(self: *Self, allocator: Allocator, reg: ComponentRegistry) void {
    for (self.columns.values()) |column| {
        const info = reg.get(column.component_id);
        if (info.deinit) |deinitFn| {
            for (0..self.len) |row| deinitFn(column.data + row * info.size, allocator);
        }
        allocator.rawFree(
            column.data[0 .. self.capacity * info.size],
            info.alignment,
            @returnAddress(),
        );
    }
    self.columns.deinit(allocator);
    allocator.free(self.entities[0..self.capacity]);
}

/// Makes sure that the table has enough capacity to store a total of `new_capacity` elements
/// without reallocating.
pub fn ensureTotalCapacity(self: *Self, a: Allocator, reg: ComponentRegistry, new_capacity: usize) Allocator.Error!void {
    if (self.capacity >= new_capacity) return;

    self.entities = @alignCast(@ptrCast(try reallocRaw(
        a,
        @ptrCast(self.entities),
        self.capacity * @sizeOf(EntitySlotIndex),
        new_capacity * @sizeOf(EntitySlotIndex),
        std.mem.Alignment.of(EntitySlotIndex),
        self.len * @sizeOf(EntitySlotIndex),
        @returnAddress(),
    )));

    errdefer self.entities = @alignCast(@ptrCast(reallocRaw(
        a,
        @ptrCast(self.entities),
        new_capacity * @sizeOf(EntitySlotIndex),
        self.capacity * @sizeOf(EntitySlotIndex),
        std.mem.Alignment.of(EntitySlotIndex),
        self.len * @sizeOf(EntitySlotIndex),
        @returnAddress(),
    ) catch cantRecover()));

    const columns = self.columns.values();
    var reallocated_count: usize = 0;

    errdefer {
        while (reallocated_count > 0) {
            reallocated_count -= 1;
            const column = &columns[reallocated_count];
            const info = reg.get(column.component_id);
            column.data = reallocRaw(
                a,
                column.data,
                new_capacity * info.size,
                self.capacity * info.size,
                info.alignment,
                self.len * info.size,
                @returnAddress(),
            ) catch cantRecover();
        }
    }

    while (reallocated_count < columns.len) {
        const column = &columns[reallocated_count];
        const info = reg.get(column.component_id);
        column.data = try reallocRaw(
            a,
            column.data,
            self.capacity * info.size,
            new_capacity * info.size,
            info.alignment,
            self.len * info.size,
            @returnAddress(),
        );
        reallocated_count += 1;
    }

    // Success!

    self.capacity = new_capacity;
}

/// Performs a reallocation.
fn reallocRaw(
    a: Allocator,
    old_ptr: [*]u8,
    old_cap: usize,
    new_cap: usize,
    alignment: std.mem.Alignment,
    copy_len: usize,
    return_address: usize,
) Allocator.Error![*]u8 {
    if (old_cap == 0) {
        return a.rawAlloc(new_cap, alignment, return_address) orelse error.OutOfMemory;
    }

    if (new_cap == 0) {
        a.rawFree(old_ptr[0..old_cap], alignment, return_address);
        return @ptrFromInt(alignment.toByteUnits());
    }

    // Note: can't set shrunk memory to undefined as memory shouldn't be modified on realloc failure
    if (a.rawRemap(old_ptr[0..old_cap], alignment, new_cap, return_address)) |p| {
        return p;
    }

    const new_ptr = a.rawAlloc(new_cap, alignment, return_address) orelse return error.OutOfMemory;
    @memcpy(new_ptr[0..copy_len], old_ptr[0..copy_len]);
    @memset(old_ptr[0..old_cap], undefined);
    a.rawFree(old_ptr[0..old_cap], alignment, return_address);
    return new_ptr;
}

fn cantRecover() noreturn {
    std.debug.panic("failed to recover after allocation failure", .{});
}

/// Makes sure that the table has enough capacity to store `additional` elements without
/// reallocating.
///
/// This method may request more memory than requested to ensure that the table does not
/// allocate too often.
pub fn ensureUnusedCapacity(
    self: *Self,
    a: Allocator,
    reg: ComponentRegistry,
    additional: usize,
) Allocator.Error!void {
    const min_cap = std.math.add(usize, self.len, additional) catch return error.OutOfMemory;
    if (min_cap > self.capacity) {
        const amortized = self.capacity *| 2;
        const new_cap = @max(min_cap, amortized, 4);
        try self.ensureTotalCapacity(a, reg, new_cap);
    }
}

/// Adds one element to the table, assuming that there is enough capacity.
pub fn addOneAssumeCapacity(self: *Self) void {
    assert(self.len < self.capacity);
    self.len += 1;
}

/// Removes one element from the table, and moves the last element to the
/// removed element's position.
///
/// # Valid Usage
///
/// This method assumes that the table is not empty and that the row is
/// within the bounds of the table.
pub fn swapRemove(self: *Self, reg: ComponentRegistry, row: usize) void {
    assert(row < self.len);

    self.len -= 1;
    if (self.len == row) return;

    for (self.columns.values()) |column| {
        const info = reg.get(column.component_id);
        @memcpy(
            column.data[row * info.size .. (row + 1) * info.size],
            column.data + self.len * info.size,
        );
    }

    self.entities[row] = self.entities[self.len];
}
