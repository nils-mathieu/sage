//! The sage engine framework.
//!
//! This can be used depended on when creating headless components for the Sage ecosystem.

pub use {
    sage_core::{
        TypeUuid, Uuid,
        app::{App, FromApp},
    },
    sage_winit::{STARTUP_SCHEDULE, UPDATE_SCHEDULE, run},
};
