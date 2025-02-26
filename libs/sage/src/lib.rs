//! The sage engine framework.
//!
//! This can be used depended on when creating headless components for the Sage ecosystem.

pub use {
    sage_core::{
        TypeUuid, Uuid,
        app::{App, Commands, FromApp},
        entities::EntityId,
    },
    sage_winit::{
        EventLoopCommands, STARTUP_SCHEDULE, UPDATE_SCHEDULE, WindowAttributes,
        dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size},
        run,
    },
};
