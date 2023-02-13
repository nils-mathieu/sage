//! Abstracts the underlying operating system and window manager, letting the user to create a
//! surface (usually a window, but not always) on which to render things.
//!
//! Specifically, this crate defines how to represent the state and the transition of popular input
//! devices such as the [keyboard](device), the [mouse](device), or [gamepads](device).
//!
//! # Application Lifecycle
//!
//! The application lifecycle is represented by the [`App`](app::App) trait. It is responsible for
//! describing how events sent by the operating system should be handled, and when to stop the
//! application.
//!
//! Rather than having a single central "Event" type, this trait defines multiple methods that
//! will be called according to the event type.
//!
//! More on that in the documentation for [app].
//!
//! # Examples
//!
//! ```no_run
//! use sage_platform::app::{App, Ctx, Tick, Config};
//! use sage_platform::device::{DeviceId, MouseButton};
//!
//! struct MyApp {
//!     close_requested: bool,
//! }
//!
//! impl App for MyApp {
//!     type Error = std::convert::Infallible;
//!     type Args = ();
//!     type Output = ();
//!
//!     fn create(_: Self::Args, _: &Ctx) -> Result<Self, Self::Error> {
//!         Ok(Self {
//!            close_requested: false,
//!         })
//!     }
//!
//!     fn close_request(&mut self, _: &Ctx) {
//!         self.close_requested = true;
//!     }
//!
//!     fn mouse_button(&mut self, _: &Ctx, _: DeviceId, btn: MouseButton, pressed: bool) {
//!         if btn == MouseButton::Left && pressed {
//!             println!("Hello, World!");
//!         }
//!     }
//!
//!     fn tick(&mut self, _: &Ctx) -> Tick<Self::Output> {
//!         if self.close_requested {
//!             Tick::Stop(())
//!         } else {
//!             Tick::Block
//!         }
//!     }
//! }
//!
//! MyApp::run((), &Config::default()).unwrap();
//! ```

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod app;
pub mod device;

mod error;

pub use error::*;

#[cfg(target_os = "windows")]
#[path = "platforms/windows/mod.rs"]
pub mod windows;
