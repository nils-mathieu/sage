//! Provides a type-safe and low-levelish API to interact with the Vulkan API.
//!
//! This library does not try to make everything safe. Instead, its goal is to make it easy to use
//! the Vulkan API correctly and efficiently.
//!
//! # Limitations
//!
//! This crate assumes that you will only ever use *one* physical device at any given time. It is
//! primarily focused on rendering and real-time graphics.

#![warn(missing_docs)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod error;

pub use error::*;

pub mod instance;
