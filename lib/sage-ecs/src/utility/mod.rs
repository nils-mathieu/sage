//! Internal utility module.

mod guard;
pub use self::guard::*;

mod noop_hash;
pub use self::noop_hash::*;

/// Assumes that the provided condition is true.
///
/// # Safety
///
/// The caller must make sure that `condition` is always true. Undefined behavior may occur
/// otherwise.
#[cfg_attr(feature = "inline-more", inline)]
pub unsafe fn assert_unchecked(condition: bool) {
    debug_assert!(condition);

    if !condition {
        unsafe { core::hint::unreachable_unchecked() }
    }
}
