//! A simple implementation of the fold-hash hashing algorithm, specifically
//! designed for pre-aligned data.

const fold_seed: u64 = 0x243f6a8885a308d3;
const init_seed: u64 = 0x13198a2e03707344;

/// Performs folded multiplication between `x` and `y`.
fn foldedMultiply(x: u64, y: u64) u64 {
    const full = @as(u128, x) *% @as(u128, y);
    const lo: u64 = @truncate(full);
    const hi: u64 = @intCast(full >> 64);
    return lo ^ hi;
}

/// Adds the provided 128-bit word to the accumulator.
pub fn hash(acc: u64, word: u128) u64 {
    const lo: u64 = @truncate(word);
    const hi: u64 = @intCast(word >> 64);
    return foldedMultiply(acc ^ lo, fold_seed ^ hi);
}

/// Computes the hash value of a 128-bit integer.
pub fn computeHashU128(x: u128) u64 {
    return hash(init_seed, x);
}
