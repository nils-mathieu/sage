//! A simple implementation of the FxHash algorithm.
//!
//! This is taken from the Rust implementation
//! available at:A
//!
//! https://github.com/cbreeden/fxhash/tree/master

const std = @import("std");

const seed64 = 0x517cc1b727220a95;
const seed32 = 0x9e3779b9;
const rotate = 5;

/// Hashes the provided 64-bit unsigned integer with another.
pub fn hash64(x: u64, y: u64) u64 {
    return (std.math.rotl(u64, x, rotate) ^ y) *% seed64;
}

/// Hashes the provided 32-bit unsigned integer with another.
pub fn hash32(x: u32, y: u32) u32 {
    return (std.math.rotl(u32, x, rotate) ^ y) *% seed32;
}

/// Hashes the provided byte slice to an `u64` value.
///
/// This function assumes that the provided byte slice is aligned like
/// an `u64`.
pub fn hashAligned(data: []align(@alignOf(u64)) const u8) u64 {
    const word_count = @divFloor(data.len, @sizeOf(u64));
    const words = @as([*]const u64, @ptrCast(data.ptr))[0..word_count];

    var hash: u64 = 0;
    for (words) |word| hash = hash64(hash, word);

    const hashed_bytes = word_count * @sizeOf(u64);
    if (hashed_bytes == data.len) {
        var rem: u64 = 0;
        @memcpy(@as([*]u8, @ptrCast(&rem)), data[hashed_bytes..]);
        hash = hash64(hash, rem);
    }

    return hash;
}
