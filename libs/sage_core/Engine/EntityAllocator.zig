//! The entity allocator is responsible for creating new
//! `Entity` IDs.
//!
//! New `Entity` IDs can be reserved concurrently to make
//! sure that insertion of new entities can be done in parallel
//! (or at the very least planned in parallel).
//!
//! # Flushing
//!
//! When the entity allocator has pending reserved entities,
//! regular allocation and deallocation operations are not
//! available.
//!
//! The allocator must be "flushed" to promote reserved entities
//! to concrete allocated entities.
//!
//! It is possible to determine whether the allocator needs
//! to be flushed by calling `needsFlush`.
//!
//! # Thread Safety
//!
//! Some of the functions in the entity allocator are thread-safe, some
//! are not.
//!
//! See the documentation for each function to determine whether a function
//! may be called concurrently or not.
//!
//! All thread-safe functions can be called concurrently, but thread-unsafe
//! functions require exclusive access to the allocator during their execution.

const std = @import("std");
const Allocator = std.mem.Allocator;
const EntityAllocator = @This();
const oom = @import("../utility/errors.zig").oom;

/// The metadata stored in an entity's slot.
pub const Metadata = @import("../Engine.zig").EntityLocation;

/// The index of an entity's slot, responsible for storing the entity's metadata.
///
/// Unlike `Entity` IDs, slot indices are re-used after they have been
/// deallocated by the entity allocator.
pub const EntitySlotIndex = u32;

/// The generation number associated with an entity's slot.
///
/// Generations are incremented every time the entity living in the slot is
/// deallocated.
///
/// This ensures that `Entity` IDs delivered by the entity allocator are
/// always unique.
pub const Generation = u32;

/// A handle to some entity living in the engine. This is a stable index into
/// the engine, meaning that insertions, deletions, and other operations on
/// entities are guaranteed not to change any already existing entity IDs.
///
/// An `Entity` ID is always unique (relative to the engine it comes from).
pub const Entity = packed struct(Entity.Int) {
    /// The integer backing the `Entity` ID type.
    pub const Int = std.meta.Int(.unsigned, @bitSizeOf(EntitySlotIndex) + @bitSizeOf(Generation));

    /// The index of the slot that stores the entity's metadata.
    slot_index: EntitySlotIndex,
    /// The generation number of the entity's slot, at the time it
    /// was allocated.
    ///
    /// If this generation number differs from the current generation
    /// number of the slot referenced by `slot_index`, then the entity
    /// has been deallocated.
    generation: Generation,

    /// A placeholder entity ID.
    ///
    /// It is not strictly impossible for this entity ID to be ever allocated,
    /// but it is extremely unlikely.
    ///
    /// This value can be used as a placeholder value.
    pub const placeholder = Entity{
        .slot_index = std.math.maxInt(EntitySlotIndex),
        .generation = std.math.maxInt(Generation),
    };

    /// Converts the entity ID to its bit representation.
    pub inline fn toBits(self: Entity) Int {
        return @bitCast(self);
    }

    /// Returns whether this `Entity` is equal to another `Entity`.
    pub inline fn eql(self: Entity, other: Entity) bool {
        return self.toBits() == other.toBits();
    }

    /// Returns whether this `Entity` differs from another `Entity`.
    pub inline fn neq(self: Entity, other: Entity) bool {
        return self.toBits() != other.toBits();
    }

    /// Formats the entity ID into the provided writer.
    pub fn format(self: Entity, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = options;
        if (fmt.len != 0) std.fmt.invalidFmtError(fmt, Entity);
        return writer.print("{}v{}", .{ self.slot_index, self.generation });
    }

    test "format" {
        const s1 = try std.fmt.allocPrint(std.testing.allocator, "{}", .{Entity{ .slot_index = 1, .generation = 2 }});
        defer std.testing.allocator.free(s1);
        try std.testing.expectEqualStrings("1v2", s1);

        const s2 = try std.fmt.allocPrint(std.testing.allocator, "{}", .{Entity{ .slot_index = 5, .generation = 3 }});
        defer std.testing.allocator.free(s2);
        try std.testing.expectEqualStrings("5v3", s2);
    }
};

/// A slot in the entity allocator.
///
/// This slot is responsible for string:
///
/// 1. A generation number which will be incremented when the entity
///    stored in the slot is deallocated.
///
/// 2. Some metadata about the entity.
pub const Slot = struct {
    /// The current generation number of the slot.
    ///
    /// This is incremented every time the entity stored in the slot is
    /// deallocated.
    generation: Generation = 0,

    /// The metadata of the entity stored in the slot.
    metadata: Metadata = undefined,
};

/// The concrete slots that have been allocated so far.
slots: std.ArrayListUnmanaged(Slot) = .empty,

/// The list of free slot indices that can be re-used.
free_slots: std.ArrayListUnmanaged(EntitySlotIndex) = .empty,

/// The number of reserved entities so far.
///
/// # Meaning
///
/// This field contains the total number of entities that have been
/// reserved concurrently so far.
///
/// When this number is non-zero, the `slots` and `free_slots` lists cannot
/// change.
///
/// - The first `free_slots.len` reserved entities are taken from the list of
///   free slots (in reverse order).
///
/// - Entities that could not be taken from the free list instead create brand
///   new slots with indices above the current `slots.len`.
reserved: std.atomic.Value(usize) = .init(0),

/// Releases the resources that were allocated by the entity
/// allocator.
///
/// After this function has been called, the allocator must no longer
/// be used.
pub fn deinit(self: *EntityAllocator, allocator: Allocator) void {
    self.free_slots.deinit(allocator);
    self.slots.deinit(allocator);
    self.* = undefined;
}

/// Reserves one entity.
///
/// Reserved entities are not immediately allocated. One must call `flush` to
/// promote previously reserved entities to concrete slots.
///
/// # Thread Safety
///
/// This function is thread-safe. It can be called concurrently with other
/// thread-safe methods.
pub fn reserveOne(self: *EntityAllocator) Entity {
    // Bump the reserved count and determine whether overflow has occurred.
    // Note that technically, it would be possible to observe an overflown
    // reserved count on a thread if the thread that triggered the overflow
    // is preempted between the fetchAdd and the check for overflow. In practice,
    // this is unlikely to occur and the program will crash as soon as the thread
    // is resumed.
    const reserved = self.reserved.fetchAdd(1, .monotonic);
    if (reserved == std.math.maxInt(usize)) tooManyReservedEntities();

    if (reserved < self.free_slots.items.len) {
        const slot_index = self.free_slots.items[self.free_slots.items.len - 1 - reserved];
        const slot = self.slots.items[slot_index];
        return Entity{ .slot_index = slot_index, .generation = slot.generation };
    } else {
        const slot_index_usize = std.math.add(usize, reserved - self.free_slots.items.len, self.slots.items.len) catch tooManyReservedEntities();
        const slot_index = std.math.cast(EntitySlotIndex, slot_index_usize) orelse tooManyReservedEntities();
        return Entity{ .slot_index = slot_index, .generation = 0 };
    }
}

test "reserveOne" {
    var entities = EntityAllocator{};

    try std.testing.expect(!entities.needsFlush());
    const e1 = entities.reserveOne();
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, e1);
    try std.testing.expect(entities.needsFlush());
}

test "reserveOne multiple times" {
    var entities = EntityAllocator{};

    try std.testing.expect(!entities.needsFlush());
    const e1 = entities.reserveOne();
    const e2 = entities.reserveOne();
    const e3 = entities.reserveOne();
    try std.testing.expect(entities.needsFlush());
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, e1);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 0 }, e2);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 0 }, e3);
}

/// An iterator over a number of entities.
pub const Iterator = struct {
    /// A pointer to the `slots` list of the associated
    /// entity allocator.
    slots: [*]const Slot,

    /// A pointer to the last slot to return from the
    /// free list.
    last_reclaimed: [*]const EntitySlotIndex,
    /// The number of slots to return from the free list,
    /// stopping at `last_reclaimed`.
    ///
    /// Reclaimed slots are returned in reverse order,
    /// starting at `last_reclaimed + reclaimed_count` down
    /// to `last_reclaimed + 0`.
    reclaimed_count: u32,

    /// The index of the first newly allocated slot to return.
    new_slots_start: EntitySlotIndex,
    /// The index of the stopping newly allocated slot index (excluded).
    new_slots_stop: EntitySlotIndex,

    /// Returns the next entity referenced by the iterator.
    pub fn next(self: *Iterator) ?Entity {
        if (self.reclaimed_count > 0) {
            self.reclaimed_count -= 1;
            const slot_index = self.last_reclaimed[self.reclaimed_count];
            const generation = self.slots[slot_index].generation;
            return Entity{ .slot_index = slot_index, .generation = generation };
        } else if (self.new_slots_start < self.new_slots_stop) {
            const slot_index = self.new_slots_start;
            self.new_slots_start += 1;
            return Entity{ .slot_index = slot_index, .generation = 0 };
        } else {
            return null;
        }
    }
};

/// Reserves multiple entities.
///
/// Reserved entities are not immediately allocated. One must call `flush` to
/// promote previously reserved entities to concrete slots.
///
/// # Thread Safety
///
/// This function is thread-safe. It can be called concurrently with other
/// thread-safe methods.
pub fn reserveMultiple(self: *EntityAllocator, count: usize) Iterator {
    // Bump the reserved count and determine whether overflow has occurred.
    // Note that technically, it would be possible to observe an overflown
    // reserved count on a thread if the thread that triggered the overflow
    // is preempted between the fetchAdd and the check for overflow. In practice,
    // this is unlikely to occur and the program will crash as soon as the thread
    // is resumed.
    const first_reserved = self.reserved.fetchAdd(count, .monotonic);
    const last_reserved = first_reserved +% count;
    if (last_reserved < first_reserved) tooManyReservedEntities();

    // The index in the free list of the first/last slot that will be
    // reclaimed.
    // Note that reclaimed slots are taken in reverse order, meaning that
    // this is actually the end/start of the slice of reclaimed slot indices.
    const free_list_index_start: EntitySlotIndex = @intCast(self.free_slots.items.len -| first_reserved);
    const free_list_index_stop: EntitySlotIndex = @intCast(self.free_slots.items.len -| last_reserved);

    // The number of newly created slots.
    const prev_new_slots_count = first_reserved -| self.free_slots.items.len;
    const new_slots_count = last_reserved -| self.free_slots.items.len;

    const new_slots_stop_usize = std.math.add(usize, self.slots.items.len, new_slots_count) catch tooManyReservedEntities();
    const new_slots_start_usize = self.slots.items.len + prev_new_slots_count;

    const new_slots_stop = std.math.cast(EntitySlotIndex, new_slots_stop_usize) orelse tooManyReservedEntities();
    const new_slots_start: EntitySlotIndex = @intCast(new_slots_start_usize);

    return Iterator{
        .slots = self.slots.items.ptr,
        .last_reclaimed = self.free_slots.items.ptr + free_list_index_start,
        .reclaimed_count = free_list_index_stop - free_list_index_start,
        .new_slots_start = new_slots_start,
        .new_slots_stop = new_slots_stop,
    };
}

test "reserveMultiple" {
    var entities = EntityAllocator{};

    var reserved = entities.reserveMultiple(6);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, reserved.next().?);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 0 }, reserved.next().?);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 0 }, reserved.next().?);
    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 0 }, reserved.next().?);
    try expectEntityEqual(Entity{ .slot_index = 4, .generation = 0 }, reserved.next().?);
    try expectEntityEqual(Entity{ .slot_index = 5, .generation = 0 }, reserved.next().?);
    try std.testing.expectEqual(null, reserved.next());
}

test "reserveMultiple is equivalent to reserveOne" {
    var entities1 = EntityAllocator{};
    var entities2 = EntityAllocator{};

    var reserved1 = entities1.reserveMultiple(6);
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
    try expectEntityEqual(reserved1.next().?, entities2.reserveOne());
}

/// Returns whether the entity allocator needs to be flushed.
///
/// # Thread Safety
///
/// This function is *not* thread-safe. Callers must ensure that the function
/// has exclusive access to the entity allocator.
pub fn needsFlush(self: EntityAllocator) bool {
    return self.reserved.raw > 0;
}

/// Flushes the entities that were reserved so far, promoting them
/// to concerete allocated entities.
///
/// This function returns an iterator over the entities that were promoted
/// by the function.
///
/// # Thead Safety
///
/// This function is *not* thread-safe. Callers must ensure that the function
/// has exclusive access to the entity allocator.
pub fn flush(self: *EntityAllocator, allocator: Allocator) Iterator {
    const last_reclaimed_index = self.free_slots.items.len -| self.reserved.raw;
    const created_slots_usize = self.reserved.raw -| self.free_slots.items.len;

    // Determine the old and new length of the `slots` list, which correspond
    // to the range of newly created slots.
    // We know those opereations won't overflow because the entities
    // could previously be reserved and no overflow occurred then.
    const created_slots: EntitySlotIndex = @intCast(created_slots_usize);
    const new_slots_start: EntitySlotIndex = @intCast(self.slots.items.len);
    const new_slots_stop: EntitySlotIndex = new_slots_start + created_slots;

    self.slots.appendNTimes(allocator, Slot{}, created_slots) catch oom();

    // The number of reclaimed slots from the free list.
    const reclaimed_count: u32 = @intCast(self.free_slots.items.len - last_reclaimed_index);

    // Truncate the length of the free list to remove the reclaimed
    // indices.
    const last_reclaimed = self.free_slots.items.ptr + last_reclaimed_index;
    self.free_slots.items.len = last_reclaimed_index;

    self.reserved.raw = 0;
    return Iterator{
        .slots = self.slots.items.ptr,
        .last_reclaimed = last_reclaimed,
        .reclaimed_count = reclaimed_count,
        .new_slots_start = new_slots_start,
        .new_slots_stop = new_slots_stop,
    };
}

test "flush" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    try std.testing.expect(!entities.needsFlush());
    var reserved = entities.reserveMultiple(6);
    try std.testing.expect(entities.needsFlush());
    var flushed = entities.flush(std.testing.allocator);
    try std.testing.expect(!entities.needsFlush());
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try expectEntityEqual(reserved.next().?, flushed.next().?);
    try std.testing.expectEqual(null, reserved.next());
    try std.testing.expectEqual(null, flushed.next());
}

/// Allocates a new entity.
///
/// # Thread Safety
///
/// This function is *not* thread-safe. It assumes to have exclusive access
/// to the entity allocator while it executes.
///
/// # Valid Usage
///
/// The caller must ensure that the entity allocator does not need
/// to be flushed.
pub fn allocateOne(self: *EntityAllocator, allocator: Allocator) Entity {
    std.debug.assert(!self.needsFlush());
    if (self.free_slots.pop()) |slot_index| {
        const generation = self.slots.items[slot_index].generation;
        return Entity{ .slot_index = slot_index, .generation = generation };
    } else {
        const slot_index = std.math.cast(EntitySlotIndex, self.slots.items.len) orelse tooManyEntities();
        self.slots.append(allocator, Slot{}) catch oom();
        return Entity{ .slot_index = slot_index, .generation = 0 };
    }
}

test "allocateOne" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, e1);
}

test "allocateOne reclaiming" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    entities.deallocate(std.testing.allocator, e1.slot_index);

    const e2 = entities.allocateOne(std.testing.allocator);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 1 }, e2);
}

/// Allocates a collection of entities.
///
/// # Thread Safety
///
/// This function is *not* thread safe. It assumes that it has
/// exclusive access to the entity allocator while it executes.
///
/// # Valid Usage
///
/// The caller must ensure that the entity allocator does not
/// need to be flushed.
pub fn allocateMultiple(self: *EntityAllocator, allocator: Allocator, count: usize) Iterator {
    std.debug.assert(!self.needsFlush());

    const last_reclaimed_entity = self.free_slots.items.len -| count;

    const last_reclaimed = self.free_slots.items.ptr + last_reclaimed_entity;
    const reclaimed_count: u32 = @intCast(self.free_slots.items.len - last_reclaimed_entity);

    const new_entities = count -| self.free_slots.items.len;
    const last_new_entity_usize = std.math.add(usize, self.slots.items.len, new_entities) catch tooManyEntities();
    const last_new_entity = std.math.cast(EntitySlotIndex, last_new_entity_usize) orelse tooManyEntities();
    const first_new_entity: EntitySlotIndex = @intCast(self.slots.items.len);

    self.slots.appendNTimes(allocator, Slot{}, new_entities) catch oom();
    self.free_slots.items.len = last_reclaimed_entity;

    return Iterator{
        .slots = self.slots.items.ptr,
        .last_reclaimed = last_reclaimed,
        .reclaimed_count = reclaimed_count,
        .new_slots_start = first_new_entity,
        .new_slots_stop = last_new_entity,
    };
}

test "allocateMultiple" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    var allocated = entities.allocateMultiple(std.testing.allocator, 6);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, allocated.next().?);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 0 }, allocated.next().?);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 0 }, allocated.next().?);
    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 0 }, allocated.next().?);
    try expectEntityEqual(Entity{ .slot_index = 4, .generation = 0 }, allocated.next().?);
    try expectEntityEqual(Entity{ .slot_index = 5, .generation = 0 }, allocated.next().?);
    try std.testing.expectEqual(null, allocated.next());
}

test "allocateMultiple reclaiming only" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    var initial_allocations = entities.allocateMultiple(std.testing.allocator, 5);
    while (initial_allocations.next()) |entt| entities.deallocate(std.testing.allocator, entt.slot_index);

    var reclaimed_allocations = entities.allocateMultiple(std.testing.allocator, 5);
    try expectEntityEqual(Entity{ .slot_index = 4, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 1 }, reclaimed_allocations.next().?);
    try std.testing.expectEqual(null, reclaimed_allocations.next());
}

test "allocateMultiple reclaiming and new at once" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    var initial_allocations = entities.allocateMultiple(std.testing.allocator, 5);
    while (initial_allocations.next()) |entt| entities.deallocate(std.testing.allocator, entt.slot_index);

    var reclaimed_allocations = entities.allocateMultiple(std.testing.allocator, 10);
    try expectEntityEqual(Entity{ .slot_index = 4, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 1 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 5, .generation = 0 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 6, .generation = 0 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 7, .generation = 0 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 8, .generation = 0 }, reclaimed_allocations.next().?);
    try expectEntityEqual(Entity{ .slot_index = 9, .generation = 0 }, reclaimed_allocations.next().?);
    try std.testing.expectEqual(null, reclaimed_allocations.next());
}

/// Deallocates the entity currently stored in the slot referenced
/// by the provided slot index.
///
/// # Thread Safety
///
/// This function is *not* thread safe. It assumes that it has
/// exclusive access to the entity allocator while it executes.
///
/// # Valid Usage
///
/// - The caller must ensure that the slot index is valid. It must have
///   been allocated previously. Attempting to free a slot that is not
///   currently allocated will result in unspecified behavior of the
///   allocator.
///
/// - The caller must ensure that the entity allocator does not need
///   to be flushed.
pub fn deallocate(self: *EntityAllocator, allocator: Allocator, slot_index: EntitySlotIndex) void {
    std.debug.assert(!self.needsFlush());
    self.free_slots.append(allocator, slot_index) catch oom();
    const slot = &self.slots.items[slot_index];
    slot.metadata = undefined;
    slot.generation = std.math.add(Generation, slot.generation, 1) catch tooManyEntities();
}

test "deallocate reclaim simple" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, e1);

    entities.deallocate(std.testing.allocator, e1.slot_index);

    const e2 = entities.allocateOne(std.testing.allocator);
    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 1 }, e2);
}

test "deallocate reclaim complicated" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    const e2 = entities.allocateOne(std.testing.allocator);
    const e3 = entities.allocateOne(std.testing.allocator);
    const e4 = entities.allocateOne(std.testing.allocator);
    const e5 = entities.allocateOne(std.testing.allocator);

    try expectEntityEqual(Entity{ .slot_index = 0, .generation = 0 }, e1);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 0 }, e2);
    try expectEntityEqual(Entity{ .slot_index = 2, .generation = 0 }, e3);
    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 0 }, e4);
    try expectEntityEqual(Entity{ .slot_index = 4, .generation = 0 }, e5);

    entities.deallocate(std.testing.allocator, e2.slot_index);
    entities.deallocate(std.testing.allocator, e4.slot_index);

    const e6 = entities.allocateOne(std.testing.allocator);
    const e7 = entities.allocateOne(std.testing.allocator);

    try expectEntityEqual(Entity{ .slot_index = 3, .generation = 1 }, e6);
    try expectEntityEqual(Entity{ .slot_index = 1, .generation = 1 }, e7);
}

/// Resolves the provided slot index to a valid entity.
///
/// # Thread Safety
///
/// This function is *not* thread-safe. It assumes no other thread will attempt
/// to modify the allocator's state while it executes, including reserving
/// entities.
///
/// # Valid Usage
///
/// - This function assumes that the allocator is currently flushed.
///
/// - The provided entity slot index must correspond to a live entity.
pub fn resolveSlotAssumeFlushed(self: EntityAllocator, slot_index: EntitySlotIndex) Entity {
    std.debug.assert(!self.needsFlush());
    const generation = self.slots.items[slot_index].generation;
    return Entity{ .slot_index = slot_index, .generation = generation };
}

test "resloveSlotAssumeFlushed" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    const resolved_e1 = entities.resolveSlotAssumeFlushed(e1.slot_index);
    try expectEntityEqual(e1, resolved_e1);
}

/// Resolves the provided slot index to a valid entity.
///
/// # Thread Safety
///
/// This function is thread-safe. It can be called concurrently along with
/// other thread-safe methods.
///
/// # Valid Usage
///
/// This function assumes that the provided slot index corresponds to a live or
/// reserved entity.
pub fn resolveSlot(self: EntityAllocator, slot_index: EntitySlotIndex) Entity {
    if (slot_index >= self.slots.items.len) {
        // If the slot index is out of bounds, we can assume that the entity
        // has been reserved previously because the function expects the
        // caller to always provide valid (allocated or reserved) slot indices.
        // this means that we can just assume a generation of zero and not even
        // read the reserved count.
        //
        // However, in debug builds, we can still check whether the invariant
        // was respected because it's not that expansive.
        const resolved_entity = Entity{ .slot_index = slot_index, .generation = 0 };

        if (std.debug.runtime_safety and !self.contains(resolved_entity)) {
            std.debug.panic("The provided slot index `{}` is neither allocated nor reserved", .{slot_index});
        }

        return resolved_entity;
    } else {
        const generation = self.slots.items[slot_index].generation;
        return Entity{ .slot_index = slot_index, .generation = generation };
    }
}

test "resolveSlot allocated entity" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    const resolved_e1 = entities.resolveSlot(e1.slot_index);
    try std.testing.expectEqual(e1, resolved_e1);
}

test "resolveSlot reserved entity" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.reserveOne();
    const resolved_e1 = entities.resolveSlot(e1.slot_index);
    try std.testing.expectEqual(e1, resolved_e1);
}

/// Returns whether the provided entity is currently allocated.
///
/// # Thread Safety
///
/// This function is *not* thread-safe. It assumes no other thread will attempt
/// to modify the allocator's state while it executes, including reserving
/// entities.
///
/// # Valid Usage
///
/// This function assumes that the allocator is currently flushed.
pub fn containsAssumeFlushed(self: EntityAllocator, entity: Entity) bool {
    std.debug.assert(!self.needsFlush());
    return entity.slot_index < self.slots.items.len and
        self.slots.items[entity.slot_index].generation == entity.generation;
}

test "containsAssumeFlushed" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    try std.testing.expect(entities.containsAssumeFlushed(e1));
    entities.deallocate(std.testing.allocator, e1.slot_index);
    try std.testing.expect(!entities.containsAssumeFlushed(e1));
}

/// Returns whether the provided entity is currently allocated or reserved.
///
/// # Thread Safety
///
/// This function is thread-safe. It may be called concurrently with other
/// operations on the allocator.
pub fn contains(self: EntityAllocator, entity: Entity) bool {
    if (entity.slot_index >= self.slots.items.len) {
        if (entity.generation != 0) return false;
        const reserved_count = self.reserved.load(.monotonic);
        const new_entities_count = reserved_count -| self.free_slots.items.len;
        return entity.slot_index < self.slots.items.len + new_entities_count;
    } else {
        return self.slots.items[entity.slot_index].generation == entity.generation;
    }
}

test "contains reserved entity" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.reserveOne();
    try std.testing.expect(entities.contains(e1));
    _ = entities.flush(std.testing.allocator);
    try std.testing.expect(entities.contains(e1));
    entities.deallocate(std.testing.allocator, e1.slot_index);
    try std.testing.expect(!entities.contains(e1));
}

test "contains allocated entity" {
    var entities = EntityAllocator{};
    defer entities.deinit(std.testing.allocator);

    const e1 = entities.allocateOne(std.testing.allocator);
    try std.testing.expect(entities.contains(e1));
    entities.deallocate(std.testing.allocator, e1.slot_index);
    try std.testing.expect(!entities.contains(e1));
}

/// Returns the metadata of the entity at the given slot index.
pub fn getMetadata(self: EntityAllocator, slot_index: EntitySlotIndex) Metadata {
    return self.slots.items[slot_index].metadata;
}

/// Returns a pointer to the metadata of the entity at the given slot index.
pub fn getMetadataPtr(self: EntityAllocator, slot_index: EntitySlotIndex) *Metadata {
    return &self.slots.items[slot_index].metadata;
}

fn tooManyReservedEntities() noreturn {
    std.debug.panic("EntityAllocator: too many reserved entities", .{});
}

fn tooManyEntities() noreturn {
    std.debug.panic("EntityAllocator: too many entities", .{});
}

fn expectEntityEqual(expected: Entity, actual: Entity) !void {
    if (expected.neq(actual)) {
        std.debug.print("Expected entity {}, got {}\n", .{ expected, actual });
        return error.TestExpectedEqual;
    }
}
