//! Atomic fragment of an entity.
//!
//! # Rust types
//!
//! In [`sage-ecs`](crate), components are not necessarily Rust types. Instead, they are just a
//! collection of bytes with an associated drop function. This allows components to come from any
//! language or runtime.

mod registry;
pub use self::registry::*;

mod bundle;
pub use self::bundle::*;

/// A trait for component types.
#[cfg(feature = "rust-components")]
pub trait Component: 'static {}
