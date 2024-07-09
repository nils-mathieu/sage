//! The entity component system powering the Sage game engine.
//!
//! # Overview
//!
//! Sage is a game engine that uses an entity component system (ECS) to manage game state
//! and logic. This crate provides a low-level container responsible for storing the
//! components and entities that make up the game world.
//!
//! # Goals and non-goals
//!
//! The main goal of this library is not to be easy-to-use or convenient. Instead, it
//! focuses on speed and flexibility and is expected to be used as a building block for higher-level
//! abstractions.
//!
//! Specifically, this library deliberately does not provide:
//!
//! - Querying or iteration over entities and components
//! - Serialization or deserialization
//! - Thread safety
//! - System scheduling
//!
//! Instead, it focuses on fast and efficient storage of entities and their components.

#![no_std]

extern crate alloc;

pub mod component;
pub mod entity;
pub mod sparse_set;
pub mod tables;

mod utility;
