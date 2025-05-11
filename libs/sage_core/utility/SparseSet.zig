const std = @import("std");
const UnmanagedArrayList = std.ArrayListUnmanaged;
const Allocator = std.mem.Allocator;
const assert = std.debug.assert;

/// Creates a sparse-set type.
///
/// # Parameters
///
/// - `T`: The value-type of the map.
///
/// - `S`: The sparse-set type for the map. This will limit the number of elements that can be
///   stored in the map, but smaller values will reduce the memory footprint of the map.
pub fn SparseSet(comptime T: type, comptime S: type) type {
    return struct {
        const Self = @This();

        /// The type of the value stored in the map. The values in this list are indexed by the
        /// values in the sparse list.
        dense: UnmanagedArrayList(T) = .{},
        /// The list of sparse indices.
        sparse: []S = &.{},

        /// A sentinel value that the sparse list will use to indicate that a value is not
        /// present in the list.
        const sentinel = std.math.maxInt(S);

        /// Releases the resources used by the sparse-set.
        ///
        /// # Valid Usage
        ///
        /// The caller must make sure that the sparse-set is not used anymore
        /// before calling this function.
        pub fn deinit(self: *Self, a: Allocator) void {
            self.dense.deinit(a);
            a.free(self.sparse);
        }

        /// Makes sure that the sparse list has enough capacity store a specific key.
        pub fn ensureSparseKey(self: *Self, a: Allocator, key: usize) Allocator.Error!void {
            assert(key != std.math.maxInt(usize));
            const old_len = self.sparse.len;
            if (old_len > key) return;
            self.sparse = try a.realloc(self.sparse, key + 1);
            @memset(self.sparse[old_len..], sentinel);
        }

        /// Makes sure that the dense list has enough capacity to store `count` additional items
        /// without reallocating.
        ///
        /// # Remarks
        ///
        /// This function will check whether adding that many elements in the set will cause
        /// the sparse index count to overflow.
        pub fn ensureUnusedCapacity(self: *Self, a: Allocator, count: usize) Allocator.Error!void {
            if (self.dense.items.len +| count >= sentinel) return error.OutOfMemory;
            return self.dense.ensureUnusedCapacity(a, count);
        }

        /// Puts a value into the sparse-set.
        ///
        /// # Valid Usage
        ///
        /// The caller must make sure that the sparse-set has enough sparse memory to store the key
        /// and value.
        pub fn putAssumeCapacity(self: *Self, key: usize, value: T) void {
            self.sparse[key] = @intCast(self.dense.items.len);
            self.dense.appendAssumeCapacity(value);
        }

        /// Inserts a value into the sparse-set.
        pub fn put(self: *Self, a: Allocator, key: usize, value: T) Allocator.Error!void {
            try self.ensureSparseKey(a, key);
            try self.ensureUnusedCapacity(a, 1);
            self.putAssumeCapacity(key, value);
        }

        /// Returns the values stored in the sparse-set.
        pub fn values(self: Self) []T {
            return self.dense.items;
        }

        /// Returns the value stored at the provided key.
        ///
        /// # Returns
        ///
        /// The value stored at the provided key, or `null` if the key is not present in the
        /// sparse-set.
        pub fn get(self: Self, key: usize) ?T {
            if (key >= self.sparse.len) return null;
            const index = self.sparse[key];
            if (index == sentinel) return null;
            return self.dense.items[index];
        }

        /// Returns the value stored at the provided key.
        ///
        /// # Valid Usage
        ///
        /// This function will not check whether the key is present in the sparse-set.
        pub fn getAssumePresent(self: Self, key: usize) T {
            assert(key < self.sparse.len);
            const index = self.sparse[key];
            assert(index != sentinel);
            return self.dense.items[index];
        }
    };
}
