const std = @import("std");
const Allocator = std.mem.Allocator;

/// Creates a "sparse set" data structure.
///
/// The created data structure maps `usize` values to a `V` by using
/// a sparse array of type `I`.
///
/// Using a sparse set is only useful if `@sizeOf(V) << @sizeOf(I)`
/// because the memory overhead of the sparse set is in `O(N + k)` where
/// `N` is the number of `V` elements in the set, and `k` is the maximum
/// value of the keys used to index into the map.
pub fn SparseSet(comptime V: type, comptime I: type) type {
    return struct {
        const Self = @This();

        /// The sparse array of the set.
        ///
        /// This is indexed by the sparse set's keys (of type `usize`)
        /// and contains the indices of the element matched by the
        /// key in the dense array.
        ///
        /// The sentinel value `sentinel` is used to represent
        /// an empty slot in the sparse array.
        sparse: []I = &.{},

        /// The dense array of the set.
        ///
        /// This contains the actual values of the set.
        dense: std.ArrayListUnmanaged(V) = .empty,

        /// The value used in the sparse array to represent an empty slot.
        const sentinel = std.math.maxInt(I);

        /// Releases the memory used by the sparse set.
        pub fn deinit(self: *Self, allocator: Allocator) void {
            self.dense.deinit(allocator);
            allocator.free(self.sparse);
        }

        /// Makes sure that the sparse array has enough capacity to hold
        /// a value at the given key.
        ///
        /// # Valid Usage
        ///
        /// The provided key cannot be the maximum value of a `usize`. Note that this would fail
        /// anyway because the system won't have enough memory. But specifically giving a key
        /// that large will trigger undefined behavior.
        pub fn ensureCapacityForKey(self: *Self, allocator: Allocator, key: usize) Allocator.Error!void {
            std.debug.assert(key != std.math.maxInt(usize));
            if (self.sparse.len <= key) {
                const prev_len = self.sparse.len;
                self.sparse = try allocator.realloc(self.sparse, key + 1);
                @memset(self.sparse[prev_len..], sentinel);
            }
        }

        /// Makes sure that the dense array has enough capacity to hold
        /// `count` additional elements.
        pub fn ensureUnusedValueCapacity(self: *Self, allocator: Allocator, count: usize) Allocator.Error!void {
            return self.dense.ensureUnusedCapacity(allocator, count);
        }

        /// Puts a new value in the set.
        ///
        /// # Valid Usage
        ///
        /// This function assumes that:
        ///
        /// - The collection has enough capacity to hold the new value at the
        ///   given key.
        ///
        /// - The collection does not already have an element at the given key.
        pub fn putNoClobberAssumeCapacity(self: *Self, key: usize, value: V) void {
            std.debug.assert(key < self.sparse.len);
            std.debug.assert(self.sparse[key] == sentinel);
            std.debug.assert(self.dense.items.len != sentinel);
            self.sparse[key] = @intCast(self.dense.items.len);
            self.dense.appendAssumeCapacity(value);
        }

        /// Returns the value at the given key, assuming that it is present.
        pub fn getAssumePresent(self: Self, key: usize) V {
            const sparse_index = self.sparse[key];
            std.debug.assert(sparse_index != sentinel);
            return self.dense.items[sparse_index];
        }

        /// Returns the value at the given key, or null if it is not present.
        pub fn get(self: Self, key: usize) ?V {
            if (key >= self.sparse.len) return null;
            const sparse_index = self.sparse[key];
            if (sparse_index == sentinel) return null;
            return self.dense.items[sparse_index];
        }

        /// Returns a slice over the values of the set.
        pub fn values(self: Self) []V {
            return self.dense.items;
        }
    };
}
