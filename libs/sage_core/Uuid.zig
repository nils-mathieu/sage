//! A Universal Unique Identifier (UUID) implementation.

const std = @import("std");
const fxhash = @import("utility/fxhash.zig");
const Uuid = @This();

/// The underlying data making up
/// the represented UUID.
data: u128,

// ============================================================================
// Comparisons
// ============================================================================

/// Returns whether this UUID is equal to another.
pub inline fn eql(self: Uuid, other: Uuid) bool {
    return self.data == other.data;
}

/// Returns whether this UUID is not equal to another.
pub inline fn neq(self: Uuid, other: Uuid) bool {
    return self.data != other.data;
}

// ============================================================================
// Parsing and formatting
// ============================================================================

/// An error that might occur while parsing a UUID string.
pub const ParseError = error{InvalidUuidFormat};

/// Parses the provided UUID string into a `Uuid` instance.
///
/// # Supported Formats
///
/// The provided string may be in one of the following formats:
///
/// - `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
/// - `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
pub fn parse(raw: []const u8) ParseError!Uuid {
    return switch (raw.len) {
        32 => parseSimple(raw.ptr),
        36 => parseHyphenated(raw.ptr),
        else => error.InvalidUuidFormat,
    };
}

/// Parses the provided value at comptile time, emitting a compiler
/// error if the operation fails.
pub fn comptimeParse(comptime raw: []const u8) Uuid {
    return parse(raw) catch {
        @compileError(std.fmt.comptimePrint(
            \\The UUID string `{s}` could not be parsed.
            \\
            \\Supported formats:
            \\- `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
            \\- `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
        , .{raw}));
    };
}

/// Parses the provided character into a 4-bit nibble.
///
/// Invalid characters are turned into an error.
fn parseNibble(c: u8) ParseError!u4 {
    return switch (c) {
        '0'...'9' => @intCast(c - '0'),
        'a'...'f' => @intCast(c - 'a' + 10),
        'A'...'F' => @intCast(c - 'A' + 10),
        else => error.InvalidUuidFormat,
    };
}

test "parseNibble" {
    const pairs: []const std.meta.Tuple(&.{ u8, u4 }) = &.{
        .{ '0', 0 },
        .{ '1', 1 },
        .{ '2', 2 },
        .{ '3', 3 },
        .{ '4', 4 },
        .{ '5', 5 },
        .{ '6', 6 },
        .{ '7', 7 },
        .{ '8', 8 },
        .{ '9', 9 },
        .{ 'A', 10 },
        .{ 'B', 11 },
        .{ 'C', 12 },
        .{ 'D', 13 },
        .{ 'E', 14 },
        .{ 'F', 15 },
        .{ 'a', 10 },
        .{ 'b', 11 },
        .{ 'c', 12 },
        .{ 'd', 13 },
        .{ 'e', 14 },
        .{ 'f', 15 },
    };

    for (pairs) |pair| {
        try std.testing.expectEqual(pair[1], try parseNibble(pair[0]));
    }
}

test "parseNibble invalid character" {
    try std.testing.expectError(error.InvalidUuidFormat, parseNibble('g'));
    try std.testing.expectError(error.InvalidUuidFormat, parseNibble('G'));
    try std.testing.expectError(error.InvalidUuidFormat, parseNibble(' '));
}

/// Parses the provided hexadecimal string into a UUID.
///
/// # Valid Usage
///
/// - `s` must reference exactly 32 bytes.
fn parseSimple(s: [*]const u8) ParseError!Uuid {
    var data: u128 = 0;
    for (s[0..32]) |c| {
        const nibble = try parseNibble(c);
        data <<= 4;
        data |= nibble;
    }
    return Uuid{ .data = data };
}

test "parseSimple" {
    const uuid = try parseSimple("0123456789abcdef0123456789abcdef");
    try std.testing.expectEqual(0x0123456789abcdef0123456789abcdef, uuid.data);
}

test "parseSimple invalid character" {
    const err = parseSimple("0123456789abcdeg0123456789abcdef");
    try std.testing.expectError(error.InvalidUuidFormat, err);
}

/// Parses the provided hyphenated UUID string.
///
/// # Valid Usage
///
/// The caller must ensure that the provided string pointer references exactly 36 bytes.
fn parseHyphenated(s: [*]const u8) ParseError!Uuid {
    // Make sure that all hyphens are in the correct places.
    if (s[8] != '-' or
        s[13] != '-' or
        s[18] != '-' or
        s[23] != '-')
    {
        return error.InvalidUuidFormat;
    }

    // Positions of a four-byte continuous hex half-word.
    // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    const positions: []const u8 = &.{ 0, 4, 9, 14, 19, 24, 28, 32 };

    var data: u128 = 0;
    for (positions) |pos| {
        const n1 = try parseNibble(s[pos]);
        const n2 = try parseNibble(s[pos + 1]);
        const n3 = try parseNibble(s[pos + 2]);
        const n4 = try parseNibble(s[pos + 3]);

        data <<= 4;
        data |= n1;
        data <<= 4;
        data |= n2;
        data <<= 4;
        data |= n3;
        data <<= 4;
        data |= n4;
    }

    return Uuid{ .data = data };
}

test "parseHyphenated" {
    const uuid = try parseHyphenated("01234567-89ab-cdef-0123-456789abcdef");
    try std.testing.expectEqual(0x0123456789abcdef0123456789abcdef, uuid.data);
}

test "parseHyphenated invalid character" {
    const err = parseHyphenated("01234567-89ab-cdef-0123-456789abcdeg");
    try std.testing.expectError(error.InvalidUuidFormat, err);
}

test "parseHyphenated missing hyphen" {
    const err = parseHyphenated("01234567-89ab-cdefa0123-456789abcdeg");
    try std.testing.expectError(error.InvalidUuidFormat, err);
}

fn invalidFormatUsage(fmt: []const u8) noreturn {
    @compileError(std.fmt.comptimePrint(
        \\Can't format a UUID using the format string `{s}`.
        \\
        \\Supported modifiers are:
        \\- `h`: Lowercase hyphenated format (default)
        \\- `H`: Uppercase hyphenated format
        \\- `s`: Lowercase simple (non-hyphenated) format
        \\- `S`: Uppercase simple (non-hyphenated) format
    , .{fmt}));
}

/// Formats a UUID string into the provided writer implementation.
pub fn format(self: Uuid, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    if (fmt.len > 1) {
        invalidFormatUsage(fmt);
    }

    var hyphenated: bool = undefined;
    var case: std.fmt.Case = undefined;

    if (fmt.len == 0) {
        hyphenated = true;
        case = .lower;
    } else {
        switch (fmt[0]) {
            'h' => {
                hyphenated = true;
                case = .lower;
            },
            'H' => {
                hyphenated = true;
                case = .upper;
            },
            's' => {
                hyphenated = false;
                case = .lower;
            },
            'S' => {
                hyphenated = false;
                case = .upper;
            },
            else => invalidFormatUsage(fmt),
        }
    }

    if (hyphenated) {
        var buf: [36]u8 = undefined;

        // Write the hyphens to the buffer.
        buf[8] = '-';
        buf[13] = '-';
        buf[18] = '-';
        buf[23] = '-';

        // Write the hexadecimal number in reverse using groups of
        // four.
        const positions: []const u8 = &.{ 0, 4, 9, 14, 19, 24, 28, 32 };
        var rem = self.data;
        var i = positions.len;
        while (i > 0) {
            i -= 1;
            const p = positions[i];
            buf[p + 3] = std.fmt.digitToChar(@intCast(rem & 0xF), case);
            rem >>= 4;
            buf[p + 2] = std.fmt.digitToChar(@intCast(rem & 0xF), case);
            rem >>= 4;
            buf[p + 1] = std.fmt.digitToChar(@intCast(rem & 0xF), case);
            rem >>= 4;
            buf[p] = std.fmt.digitToChar(@intCast(rem & 0xF), case);
            rem >>= 4;
        }

        return std.fmt.formatBuf(&buf, options, writer);
    } else {
        var buf: [32]u8 = undefined;

        var i: usize = 32;
        var rem = self.data;
        while (i > 0) {
            i -= 1;
            buf[i] = std.fmt.digitToChar(@intCast(rem & 0xF), case);
            rem >>= 4;
        }

        return std.fmt.formatBuf(&buf, options, writer);
    }
}

test "formatHyphenated" {
    const uuid: Uuid = try parse("01234567-89ab-cdef-0123-456789abcdef");

    const s1 = try std.fmt.allocPrint(std.testing.allocator, "{}", .{uuid});
    defer std.testing.allocator.free(s1);
    try std.testing.expectEqualStrings("01234567-89ab-cdef-0123-456789abcdef", s1);

    const s2 = try std.fmt.allocPrint(std.testing.allocator, "{h}", .{uuid});
    defer std.testing.allocator.free(s2);
    try std.testing.expectEqualStrings("01234567-89ab-cdef-0123-456789abcdef", s2);

    const s4 = try std.fmt.allocPrint(std.testing.allocator, "{H}", .{uuid});
    defer std.testing.allocator.free(s4);
    try std.testing.expectEqualStrings("01234567-89AB-CDEF-0123-456789ABCDEF", s4);
}

test "formatSimple" {
    const uuid: Uuid = try parse("01234567-89ab-cdef-0123-456789abcdef");

    const s1 = try std.fmt.allocPrint(std.testing.allocator, "{s}", .{uuid});
    defer std.testing.allocator.free(s1);
    try std.testing.expectEqualStrings("0123456789abcdef0123456789abcdef", s1);

    const s2 = try std.fmt.allocPrint(std.testing.allocator, "{S}", .{uuid});
    defer std.testing.allocator.free(s2);
    try std.testing.expectEqualStrings("0123456789ABCDEF0123456789ABCDEF", s2);
}

// ============================================================================
// Hashing
// ============================================================================

/// Hashes the provided UUID.
///
/// It assumes fairly good entropy from the original UUID.
pub fn hash(self: Uuid) u64 {
    const low: u64 = @truncate(self.data);
    const high: u64 = @intCast(self.data >> 64);
    return fxhash.hash64(low, high);
}

/// A zero-sized hash-map context that may be used when creating a hash-map that takes
/// a UUID as its key.
pub const HashMapContext = struct {
    pub inline fn hash(self: @This(), uuid: Uuid) u64 {
        _ = self;
        return uuid.hash();
    }

    pub inline fn eql(self: @This(), a: Uuid, b: Uuid) bool {
        _ = self;
        return a.eql(b);
    }
};

/// Returns an unmanaged hash-map type that uses a `Uuid` as key.
pub fn HashMapUnmanaged(comptime V: type, comptime max_load_percentage: u8) type {
    return std.hash_map.HashMapUnmanaged(Uuid, V, HashMapContext, max_load_percentage);
}

/// Returns a managed hash-map type that uses a `Uuid` as key.
pub fn HashMap(comptime V: type, comptime max_load_percentage: u8) type {
    return std.hash_map.HashMap(Uuid, V, HashMapContext, max_load_percentage);
}
