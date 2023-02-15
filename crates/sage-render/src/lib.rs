//! Provides a type-safe and low-levelish API to interact with the Vulkan API.
//!
//! This library does not try to make everything safe. Instead, its goal is to make it easy to use
//! the Vulkan API correctly and efficiently.
//!
//! # Limitations
//!
//! This crate assumes that you will only ever use *one* physical device at any given time. It is
//! primarily focused on rendering and real-time graphics. It tries, when possible, to avoid
//! assuming a too specific usage of the API, but if you want to use the whole capabilities of the
//! Vulkan API, you'll be better with a thinner wrapper such as **Vulkano**, or even **ash** (on
//! which this crate is built).

#![warn(missing_docs)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod error;

pub use error::*;

pub mod gpu;
pub mod instance;
