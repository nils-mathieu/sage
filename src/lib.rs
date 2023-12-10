//! An entity component system library written in Rust.

#![no_std]
#![cfg_attr(feature = "nightly", feature(strict_provenance))]

pub mod entities;
pub mod query;

extern crate alloc;

mod world;
pub use world::*;

pub use entities::{Component, Entity};
