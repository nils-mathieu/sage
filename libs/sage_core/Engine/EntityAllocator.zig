//! Allows allocating `Entity` IDs, eventually concurrently.

/// The index of a slot within the `EntityAllocator`.
///
/// This is used to identify a slot in the entity allocator.
pub const SlotIndex = u32;

/// The generation of a slot within the `EntityAllocator`.
pub const Generation = u32;

/// A cheap-to-copy reference to an entity managed by the Sage engine.
pub const Entity = packed struct(u64) {
    /// The index of the slot that the entity is stored in.
    slot: SlotIndex,
    /// The generation of the entity.
    generation: Generation,
};
