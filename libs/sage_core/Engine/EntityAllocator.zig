//! Allows allocating `Entity` IDs, eventually concurrently.
//!
//! # Thread Safety
//!
//! Some of the methods in this module are thread-safe, while others are not. Thread-safe methods
//! can all be called concurrently without any issues. Non-thread-safe methods can never be called
//! concurrently with thread-safe methods.
//!
//! Please refer to the documentation of each method for more details.

const std = @import("std");
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const Allocator = std.mem.Allocator;
const assert = std.debug.assert;

const Self = @This();

/// The metadata associated with an entity.
pub const Metadata = @import("../Engine.zig").EntityLocation;

/// The index of a slot within the `EntityAllocator`.
///
/// This is used to identify a slot in the entity allocator. Note that unlike regular
/// `Entity` IDs, slot indices are not guaranteed to remain unique across removal
/// and insertion of entities. Typically, despawning an entity will recycle its slot
/// and allow another entity to take its place.
///
/// For this reason, a slot index should only be stored as long as the entity it refers
/// to is alive. If the entity it originally referred to is removed, and the slot index
/// is reclaimed, then there will be no way to know whether the slot index was originally
/// used for the removed entity or the new entity that took its place.
pub const SlotIndex = u32;

/// The generation of a slot within the `EntityAllocator`.
///
/// The generation number of a slot is incremented every time the slot is reclaimed by the
/// allocator.
pub const Generation = u32;

/// A cheap-to-copy reference to an entity managed by the Sage engine.
pub const Entity = packed struct(u64) {
    /// The index of the slot that the entity is stored in.
    slot: SlotIndex = std.math.maxInt(SlotIndex),
    /// The generation of the entity.
    generation: Generation = std.math.maxInt(Generation),
};

/// Contains information about a slot in the `EntityAllocator`.
pub const Slot = struct {
    /// The generation of the slot.
    ///
    /// This must be bumped every time the slot is reclaimed.
    generation: Generation = 0,
    /// The metadata associated with the slot.
    metadata: Metadata = undefined,
};

/// The list of slots that have been allocated so far.
slots: ArrayListUnmanaged(Slot) = ArrayListUnmanaged(Slot).empty,
/// The list of free-slots indices that can be re-used.
free_slots: ArrayListUnmanaged(SlotIndex) = ArrayListUnmanaged(SlotIndex).empty,
/// The number of slots that have been reserved concurrently.
///
/// # Meaning
///
/// Reserved entities are taken in priority from the `free_slots` list, starting from the
/// end. Past that point, the allocator will start allocating new slots on top of the existing
/// `slots` list.
reserved_count: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

/// Releases the resources used by the entity allocator.
///
/// # Valid Usage
///
/// The caller must make sure that the entity allocator will no longer be used
/// after this function is called.
pub fn deinit(self: *Self, allocator: Allocator) void {
    self.slots.deinit(allocator);
    self.free_slots.deinit(allocator);
}

// =================================================================================================
// Unsynchronized entity allocation methods.
// =================================================================================================

/// Allocates a new entity ID.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes exclusive access to the entity allocator.
///
/// # Valid Usage
///
/// - The caller must make sure the allocator has pending reserved entities.
pub fn allocate(self: *Self, allocator: Allocator) Allocator.Error!Entity {
    assert(!self.hasReservedEntities());
    if (self.free_slots.pop()) |slot| {
        const generation = self.slots.items[slot].generation;
        return Entity{ .generation = generation, .slot = slot };
    } else {
        const slot = std.math.cast(SlotIndex, self.slots.items.len) orelse tooManyEntities();
        try self.slots.append(allocator, Slot{});
        return Entity{ .generation = 0, .slot = slot };
    }
}

/// Ensures that the allocator has enough capacity to reclaim `count` additional slots
/// without reallocating or failing.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes exclusive access to the entity allocator.
pub fn ensureDeallocateCapacity(self: *Self, allocator: Allocator, count: usize) Allocator.Error!void {
    return self.free_slots.ensureUnusedCapacity(allocator, count);
}

/// Deallocates the entity stored at the provided slot.
///
/// # Thread Safety
///
/// The method is *not* thread-safe. It assumes exclusive access to the entity allocator.
///
/// # Valid Usage
///
/// The caller must make sure that:
///
/// - The provided slot index is valid.
///
/// - The allocator has no pending reserved entities.
///
/// - The allocator has enough capacity to reclaim the slot. This can be ensured by calling
///   `ensureDeallocateCapacity` before calling this method.
pub fn deallocateAssumeCapacity(self: *Self, slot: SlotIndex) void {
    assert(!self.hasReservedEntities());
    assert(slot < self.slots.items.len);
    assert(std.mem.indexOfScalar(SlotIndex, self.free_slots.items, slot) == null);
    assert(self.free_slots.items.len < self.slots.capacity);
    self.slots.items[slot].generation += 1;
    self.free_slots.appendAssumeCapacity(slot);
}

/// Deallocates the entity stored at the provided slot.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes exclusive access to the entity allocator.
///
/// # Valid Usage
///
/// The caller must make sure that:
///
/// - The provided slot index is valid.
///
/// - The allocator has no pending reserved entities.
pub fn deallocate(self: *Self, allocator: Allocator, slot: SlotIndex) Allocator.Error!void {
    try self.ensureDeallocateCapacity(allocator, 1);
    self.deallocateAssumeCapacity(slot);
}

// =================================================================================================
// Entity reservation methods.
// =================================================================================================

/// An iterator type returned by the `flushReservedEntities` method.
pub const FlushedEntities = struct {
    /// The slots that have been reserved.
    slots: []const Slot,
    /// The slot indices that have been reclaimed.
    ///
    /// This is a list of slot indices that have were reclaimed during
    /// the operation.
    reclaimed: []const SlotIndex,
    /// The number of slots that have been created on top of the existing
    /// ones.
    first_created: SlotIndex,

    /// Returns the next entity that has been reserved.
    pub fn next(self: *FlushedEntities) ?Entity {
        if (self.reclaimed.len > 0) {
            const slot = self.reclaimed[self.reclaimed.len - 1];
            const generation = self.slots[slot].generation;
            self.reclaimed.len -= 1;
            return Entity{ .generation = generation, .slot = slot };
        } else if (self.first_created < self.slots.len) {
            const slot = self.first_created;
            const generation = self.slots[slot].generation;
            self.first_created += 1;
            return Entity{ .generation = generation, .slot = slot };
        } else {
            return null;
        }
    }
};

/// Flushes the entity allocator, converting pending reserved entities into proper
/// allocated entities.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes no mutation to the entity allocator
/// while it executes.
///
/// # Returns
///
/// This method returns an iterator over the entities that have been reserved during
/// the operation.
pub fn flushReservedEntities(self: *Self, allocator: Allocator) Allocator.Error!FlushedEntities {
    const reserved = self.reserved_count.raw;
    const last_reclaimed_index = self.free_slots.items.len -| reserved;
    const created_count = reserved -| self.free_slots.items.len;
    const first_created: SlotIndex = @intCast(self.slots.items.len);

    try self.slots.appendNTimes(allocator, Slot{}, created_count);
    self.free_slots.items.len = last_reclaimed_index;
    self.reserved_count.raw = 0;

    return FlushedEntities{
        .slots = self.slots.items,
        .reclaimed = self.free_slots.items[last_reclaimed_index..],
        .first_created = first_created,
    };
}

/// Returns whether the entity allocator has any pending reserved entities.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes no mutation to the entity allocator
/// while it executes.
pub fn hasReservedEntities(self: Self) bool {
    return self.reserved_count.raw > 0;
}

/// Returns the number of pending reserved entities.
///
/// # Thread Safety
///
/// This method is *not* thread-safe. It assumes no mutation to the entity allocator
/// while it executes.
pub fn reservedEntities(self: Self) usize {
    return self.reserved_count.raw;
}

/// Determines the `Entity` ID that has been reserved when the reserved count
/// has reached the provided index.
fn reservedIndexToEntity(self: *Self, reserved: usize) Entity {
    if (reserved < self.free_slots.items.len) {
        const slot = self.free_slots.items[self.free_slots.items.len - 1 - reserved];
        const generation = self.slots.items[slot].generation;
        return Entity{ .generation = generation, .slot = slot };
    } else {
        const slot_index = self.slots.items.len + (reserved - self.free_slots.items.len);
        const slot = std.math.cast(SlotIndex, slot_index) orelse tooManyEntities();
        return Entity{ .generation = 0, .slot = slot };
    }
}

/// Reserves a single entity ID.
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
pub fn reserveOne(self: *Self) Entity {
    const reserved = self.reserved_count.fetchAdd(1, .monotonic);
    if (reserved == std.math.maxInt(usize)) tooManyEntities();
    return self.reservedIndexToEntity(reserved);
}

/// The result of the `reserveMany` method. It's an iterator over the entities
/// that were reserved during the operation.
pub const ReserveMany = struct {
    /// The parent entity allocator.
    parent: *Self,
    /// The first reserved entity.
    start: usize,
    /// The first non-reserved entity.
    stop: usize,

    /// The index of the next entity that has been reserved.
    pub fn next(self: *ReserveMany) ?Entity {
        if (self.start >= self.stop) return null;
        const reserved = self.start;
        self.start += 1;
        return self.parent.reservedIndexToEntity(reserved);
    }
};

/// Reserves multiple entity IDs.
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
pub fn reserveMany(self: *Self, count: usize) Allocator.Error!ReserveMany {
    const start = self.reserved_count.fetchAdd(count, .monotonic);
    const stop = start +% count;
    if (stop < start) tooManyEntities();
    return ReserveMany{
        .parent = self,
        .start = start,
        .stop = stop,
    };
}

// =================================================================================================
// Lifecycle and query methods.
// =================================================================================================

/// Resolves the entity ID stored at the provided slot index.
///
/// # Remarks
///
/// This method may return an entity ID that has not been allocated yet, when the provided
/// slot index has already been reclaimed.
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
///
/// # Returns
///
/// If the slot index has been reserved previously, or if it is exist (though even if it has been
/// reclaimed), then the entity ID associated with the slot index is returned.
///
/// Otherwise, if the slot index has not been reserved and does not exist, then `null` is returned.
pub fn resolveSlot(self: Self, slot: SlotIndex) ?Entity {
    if (slot >= self.slots.items.len) {
        const reserved = self.reserved_count.load(.monotonic);
        if (reserved < self.free_slots.items.len or reserved - self.free_slots.items.len < slot) return null;
        return Entity{ .generation = 0, .slot = slot };
    } else {
        const generation = self.slots.items[slot].generation;
        return Entity{ .generation = generation, .slot = slot };
    }
}

/// Resolves the provided slot index to an entity ID.
///
/// # Remarks
///
/// This method may return an entity ID that has not been allocated yet, when the provided
/// slot index has already been reclaimed (through deallocation for example).
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
///
/// # Valid Usage
///
/// - This function assumes that the entity ID is valid. It must either have been allocated or
///   reserved previously.
pub fn resolveSlotAssumeValid(self: Self, slot: SlotIndex) Entity {
    if (slot >= self.slots.items.len) {
        // We are assuming that the slot is valid, so if we're out of bounds, we know for sure
        // that the entity must have been reserved.
        return Entity{ .generation = 0, .slot = slot };
    } else {
        const generation = self.slots.items[slot].generation;
        return Entity{ .generation = generation, .slot = slot };
    }
}

/// Resolves the provided slot index to an entity ID.
///
/// # Remarks
///
/// This method may return an entity ID that has not been allocated yet, when the provided
/// slot index has already been reclaimed (through deallocation for example).
///
/// # Thread Safety
///
/// This method is thread-safe and may be called concurrently along with other thread-safe
/// methods.
///
/// # Valid Usage
///
/// - This function assumes that the allocator has no pending reserved entities.
pub fn resolveSlotAssumeNoPending(self: Self, slot: SlotIndex) Entity {
    assert(self.reserved_count.load(.monotonic) == 0);
    const generation = self.slots.items[slot].generation;
    return Entity{ .generation = generation, .slot = slot };
}

/// Returns whether the provided entity ID is valid.
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
///
/// # Remarks
///
/// Entities that have been reserved are considered valid.
pub fn contains(self: Self, entity: Entity) bool {
    return if (self.resolveSlot(entity.slot)) |resolved|
        resolved.generation == entity.generation
    else
        false;
}

/// Returns whether the provided entity ID is valid.
///
/// # Thread Safety
///
/// This method is thread-safe. It may be called concurrently along with other thread-safe
/// methods.
///
/// # Valid Usage
///
/// - This function assumes that the entity allocator has no pending reserved entities.
pub fn containsAssumeNoPending(self: Self, entity: Entity) bool {
    assert(self.reserved_count.load(.monotonic) == 0);
    if (entity.slot >= self.slots.items.len) return false;
    return self.slots.items[entity.slot].generation == entity.generation;
}

// =================================================================================================
// Metadata.
// =================================================================================================

/// Returns the metadata associated with the provided entity ID.
///
/// # Valid Usage
///
/// The caller must make sure that the provided entity ID is valid and not reserved.
pub fn getMetadataPtr(self: Self, entity: SlotIndex) *Metadata {
    assert(entity < self.slots.items.len);
    return &self.slots.items[entity].metadata;
}

/// Returns the metadata associated with the provided entity ID.
///
/// # Valid Usage
///
/// The caller must make sure that the provided entity ID is valid and not reserved.
pub fn getMetadata(self: Self, entity: SlotIndex) Metadata {
    assert(entity < self.slots.items.len);
    return self.slots.items[entity].metadata;
}

// =================================================================================================
// Utility methods.
// =================================================================================================

/// Crashes the program with a message indicating that too many entities have been allocated
/// and the allocator has run out of valid IDs.
///
/// Running out of IDs is astronomically unlikely, and is probably a sign of a bug in the
/// game code (leaking entities).
fn tooManyEntities() noreturn {
    std.debug.panic("too many entities", .{});
}

// =================================================================================================
// Tests.
// =================================================================================================

test "allocateOne" {
    const a = std.testing.allocator;
    var entities: Self = Self{};
    defer entities.deinit(a);

    const e = try entities.allocate(a);
    try std.testing.expectEqual(0, e.slot);
    try std.testing.expectEqual(0, e.generation);
    try std.testing.expect(entities.contains(e));
}

test "deallocateOne" {
    const a = std.testing.allocator;
    var entities: Self = Self{};
    defer entities.deinit(a);

    const e = try entities.allocate(a);
    try std.testing.expectEqual(0, e.slot);
    try std.testing.expectEqual(0, e.generation);
    try std.testing.expect(entities.contains(e));

    try entities.deallocate(a, e.slot);
    try std.testing.expect(!entities.contains(e));
}

test "reclaimSlot" {
    const a = std.testing.allocator;
    var entities: Self = Self{};
    defer entities.deinit(a);

    const e1 = try entities.allocate(a);
    const e2 = try entities.allocate(a);
    try std.testing.expectEqual(0, e1.slot);
    try std.testing.expectEqual(0, e1.generation);
    try std.testing.expectEqual(1, e2.slot);
    try std.testing.expectEqual(0, e2.generation);
    try std.testing.expect(entities.contains(e1));
    try std.testing.expect(entities.contains(e2));

    try entities.deallocate(a, e1.slot);
    try std.testing.expect(!entities.contains(e1));

    const e3 = try entities.allocate(a);
    try std.testing.expectEqual(0, e3.slot);
    try std.testing.expectEqual(1, e3.generation);
    try std.testing.expect(entities.contains(e3));
    try std.testing.expect(!entities.contains(e1));
}

test "reserveOne" {
    const a = std.testing.allocator;
    var entities = Self{};
    defer entities.deinit(a);

    const e1 = entities.reserveOne();
    try std.testing.expectEqual(0, e1.slot);
    try std.testing.expectEqual(0, e1.generation);
    try std.testing.expect(entities.contains(e1));

    const e2 = entities.reserveOne();
    try std.testing.expectEqual(1, e2.slot);
    try std.testing.expectEqual(0, e2.generation);
    try std.testing.expect(entities.contains(e2));

    var flushed = try entities.flushReservedEntities(a);
    const re1 = flushed.next().?;
    try std.testing.expectEqual(0, re1.slot);
    try std.testing.expectEqual(0, re1.generation);
    const re2 = flushed.next().?;
    try std.testing.expectEqual(1, re2.slot);
    try std.testing.expectEqual(0, re2.generation);
    try std.testing.expectEqual(null, flushed.next());

    try std.testing.expect(entities.contains(re1));
    try std.testing.expect(entities.contains(re2));
}
