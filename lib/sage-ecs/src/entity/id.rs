use core::num::NonZero;

/// A lightweight identifier for an entity living in an [`UnsafeWorld`].
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub struct Entity {
    #[cfg(target_endian = "little")]
    index: u32,
    #[cfg(target_endian = "little")]
    generation: NonZero<u32>,

    #[cfg(target_endian = "big")]
    generation: NonZero<u32>,
    #[cfg(target_endian = "big")]
    index: u32,
}

impl Entity {
    /// Creates a new [`Entity`] identifier with the provided index and generation numbers.
    #[cfg_attr(feature = "inline-more", inline)]
    pub(super) const fn new(index: u32, generation: NonZero<u32>) -> Self {
        Self { index, generation }
    }

    /// Returns the bit-representation of the [`Entity`] identifier.
    ///
    /// This is mainly used for hashing and serialization purposes.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn to_bits(self) -> u64 {
        unsafe { core::mem::transmute(self) }
    }

    /// Creates an [`Entity`] identifier from its bit-representation without checking the generation
    /// number.
    ///
    /// # Safety
    ///
    /// The generation number of the provided bits must not be zero. This can be checked by
    /// extracting the upper 32 bits of the provided number.
    #[cfg_attr(feature = "inline-more", inline)]
    pub unsafe fn from_bits_unchecked(bits: u64) -> Self {
        unsafe { core::mem::transmute(bits) }
    }

    /// Creates an [`Entity`] identifier from its bit-representation.
    ///
    /// This is mainly used for deserialization purposes.
    ///
    /// # Panics
    ///
    /// This function panics if the generation number of the provided bits is zero.
    #[cfg_attr(feature = "inline-more", inline)]
    #[track_caller]
    pub fn from_bits(bits: u64) -> Self {
        Self::try_from_bits(bits).expect("entity generation number must not be zero")
    }

    /// Tries to create an [`Entity`] identifier from its bit-representation.
    ///
    /// Returns `None` if the generation number of the provided bits is zero.
    ///
    /// This is mainly used for deserialization purposes.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn try_from_bits(bits: u64) -> Option<Self> {
        if bits >> 32 == 0 {
            None
        } else {
            unsafe { Some(Self::from_bits_unchecked(bits)) }
        }
    }

    /// Returns the index number associated with this [`Entity`] identifier.
    ///
    /// In a given [`UnsafeWorld`], this number is unique to each *living* entities. Note that
    /// two entities can have the same index as long as they do not live at the same time. If you
    /// need to distinguish between entities that have the same index, you should use the
    /// generation number as well.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Returns the generation number associated with this [`Entity`] identifier.
    ///
    /// This number is attached to the index number of the entity and is incremented each time
    /// an entity with that index is removed from the [`UnsafeWorld`].
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn generation(&self) -> u32 {
        self.generation.get()
    }
}

impl core::fmt::Debug for Entity {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Entity({}:{})", self.index, self.generation.get())
    }
}

impl core::fmt::Display for Entity {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "<{}:{}>", self.index, self.generation.get())
    }
}

impl core::cmp::PartialEq for Entity {
    #[cfg_attr(feature = "inline-more", inline)]
    fn eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl core::cmp::Eq for Entity {}

impl core::hash::Hash for Entity {
    #[cfg_attr(feature = "inline-more", inline)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state)
    }
}

impl core::cmp::PartialOrd for Entity {
    #[cfg_attr(feature = "inline-more", inline)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::cmp::Ord for Entity {
    #[cfg_attr(feature = "inline-more", inline)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_bits().cmp(&other.to_bits())
    }
}
