use core::hash::{BuildHasher, Hasher};

/// A hash map that does not hash its keys.
pub type NoopHashMap<K, V> = hashbrown::HashMap<K, V, NoopBuildHasher>;

/// A [`BuildHasher`] implementation that creates instances of [`NoOpHasher`].
pub struct NoopBuildHasher;

impl BuildHasher for NoopBuildHasher {
    type Hasher = NoOpHasher;

    #[cfg_attr(feature = "inline-more", inline)]
    fn build_hasher(&self) -> Self::Hasher {
        NoOpHasher::new()
    }
}

/// A [`Hasher`] implementation that does not hash anything.
///
/// This is useful for types that already contain a hashed value, such as [`TypeId`] for example.
///
/// [`TypeId`]: core::any::TypeId
///
/// # Implementation
///
/// The [`Hasher`] implementation of this type panics if it is used to hash something that is not
/// `u64` or `i64`.
pub struct NoOpHasher {
    #[cfg(debug_assertions)]
    used: bool,
    hash: u64,
}

impl NoOpHasher {
    /// Creates a new [`NoOpHasher`] instance.
    pub const fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            used: false,
            hash: 0,
        }
    }
}

impl Default for NoOpHasher {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for NoOpHasher {
    fn finish(&self) -> u64 {
        #[cfg(debug_assertions)]
        {
            assert!(
                self.used,
                "NoOpHasher was not used before finishing the hash"
            );
        }

        self.hash
    }

    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("NoOpHasher should not be used to hash arbitrary bytes");
    }

    fn write_u64(&mut self, i: u64) {
        #[cfg(debug_assertions)]
        {
            assert!(!self.used, "NoOpHasher was used more than once");
            self.used = true;
        }

        self.hash = i;
    }
}
