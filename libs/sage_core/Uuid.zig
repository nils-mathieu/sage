//! A Universally Unique Identifier (UUID).

const std = @import("std");
const assert = std.debug.assert;

const fold_hash = @import("utility/fold_hash.zig");

const Self = @This();

/// The raw UUID data.
bytes: Bytes,

/// The raw byte storage of a UUID.
pub const Bytes = [16]u8;

/// Returns whether the provided two UUIDs are equal.
pub fn eql(self: Self, other: Self) bool {
    return std.mem.eql(u8, &self.bytes, &other.bytes);
}

/// Returns a hash map that uses UUIDs as keys.
pub fn MapUnmanaged(comptime T: type) type {
    const MapContext = struct {
        pub fn hash(self: @This(), x: Self) u64 {
            _ = self;
            return fold_hash.computeHashU128(x);
        }

        pub fn eql(self: @This(), x: Self, y: Self) bool {
            _ = self;
            return x.eql(y);
        }
    };

    return std.hash_map.HashMapUnmanaged(
        Self,
        T,
        MapContext,
        std.hash_map.default_max_load_percentage,
    );
}

/// An error that can be returned when a parsed UUID is invalid.
pub const InvalidUuid = error{InvalidUuid};

/// Parses the provided string representation of a UUID into
/// a UUID object.
///
/// This function supports the following formats:
///
/// - Hyphenated: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
/// - Non-hyphenated: `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
pub fn parse(s: []const u8) InvalidUuid!Self {
    switch (s.len) {
        32 => return parseNonHyphenated(s.ptr),
        36 => return parseHyphenated(s.ptr),
        else => return error.InvalidUuid,
    }
}

/// Parses the provided non-hyphenated string representation of a UUID into
/// a UUID object.
///
/// # Valid Usage
///
/// This function assumes that the provided string has a length of 32
/// characters.
fn parseNonHyphenated(s: [*]const u8) InvalidUuid!Self {
    var ret: Bytes = undefined;
    for (0..16) |i| {
        const h1 = hex_table[s[i * 2]];
        const h2 = hex_table[s[i * 2 + 1]];
        if (h1 | h2 == 0xFF) return error.InvalidUuid;
        ret[i] = (h1 << 4) | h2;
    }
    return Self{ .bytes = ret };
}

/// Parses the provided hyphenated string representation of a UUID into
/// a UUID object.
///
/// # Valid Usage
///
/// This function assumes that the provided string has a length of 36
/// characters.
fn parseHyphenated(s: [*]const u8) InvalidUuid!Self {
    if (s[8] != '-' or
        s[13] != '-' or
        s[18] != '-' or
        s[23] != '-')
        return error.InvalidUuid;

    var ret: Bytes = undefined;
    for ([_]usize{ 0, 4, 9, 14, 19, 24, 28, 32 }) |i| {
        const h1 = hex_table[s[i]];
        const h2 = hex_table[s[i + 1]];
        const h3 = hex_table[s[i + 2]];
        const h4 = hex_table[s[i + 3]];
        if (h1 | h2 | h3 | h4 == 0xFF) return error.InvalidUuid;
        ret[i * 2] = (h1 << 4) | h2;
        ret[i * 2 + 1] = (h3 << 4) | h4;
    }
    return Self{ .bytes = ret };
}

/// Converts an ASCII hex digit to a nibble.
///
/// Invalid characters are mapped to the value `0xFF`.
const hex_table = a: {
    var ret: [256]u8 = undefined;
    for (0..256) |i| {
        ret[i] = if (i >= '0' and i <= '9')
            i - '0'
        else if (i >= 'a' and i <= 'f')
            i - 'a' + 10
        else if (i >= 'A' and i <= 'F')
            i - 'A' + 10
        else
            0xFF;
    }
    break :a ret;
};
