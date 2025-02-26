#[cfg(target_pointer_width = "32")]
type UuidStorage = [u32; 4];

#[cfg(target_pointer_width = "64")]
type UuidStorage = [u64; 2];

/// A globally unique identifier for a global resource stored in a [`Globals`] collection.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uuid(UuidStorage);

impl Uuid {
    /// Creates a new [`Uuid`] instance from the provided bytes encoded as a little-endian
    /// 128-bit integer.
    #[inline]
    pub const fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self::from_u128(u128::from_le_bytes(bytes))
    }

    /// Creates a new [`Uuid`] instance from the provided bytes encoded as a big-endian
    /// 128-bit integer.
    #[inline]
    pub const fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self::from_u128(u128::from_be_bytes(bytes))
    }

    /// Creates a new [`Uuid`] instance from the provided bytes encoded as a native-endian
    /// 128-bit integer.
    #[inline]
    pub const fn from_ne_bytes(bytes: [u8; 16]) -> Self {
        Self::from_u128(u128::from_ne_bytes(bytes))
    }

    /// Creates a new [`Uuid`] instance from the provided bytes encoded as a a 128-bit
    /// integer.
    #[inline]
    pub const fn from_u128(val: u128) -> Self {
        unsafe { core::mem::transmute(val) }
    }

    /// Returns the UUID as a 128-bit integer.
    #[inline]
    pub const fn as_u128(self) -> u128 {
        unsafe { core::mem::transmute(self) }
    }
}

impl std::fmt::Debug for Uuid {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Uuid({:032x})", self.as_u128())
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.as_u128())
    }
}

/// A trait that can be implemented by types that have an associated globally unique identifier.
///
/// # Safety
///
/// The implementor must ensure that the `UUID` is actually globally unique.
///
/// This is basically impossible to enforce mathimatically, but using a proper source of randomness,
/// collisions should be unlikely enough that memory safety is not compromised in a any practical
/// sense.
///
/// I mean, if you actually end up with a collision, you'll crash and burn and change the UUID.
/// It will likely happen before actually hitting memory-critical code because UUIDs are checked for
/// uniqueness when resources are registered before you actually use them for anything else.
pub unsafe trait TypeUuid {
    /// The globally unique identifier for the type.
    const UUID: Uuid;
}
