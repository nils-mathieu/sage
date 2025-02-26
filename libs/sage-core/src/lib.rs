//! Defines the barebones core functionality of the Sage game engine.
//!
//! This does not include much, appart from state management.

#![feature(const_type_name)]
#![feature(nonnull_provenance)]

pub mod app;

mod uuid;
pub use self::uuid::*;

mod function;
pub use self::function::*;

mod opaque_ptr;
pub use self::opaque_ptr::*;

pub mod entities;
pub mod system;
