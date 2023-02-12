//! Abstracts the underlying operating system and window manager, letting the user to create a
//! surface (usually a window, but not always) on which to render things.
//!
//! Specifically, this crate defines how to represent the state and the transition of popular input
//! devices such as the [keyboard](device), the [mouse](device), or [gamepads](device).
//!
//! # Application Lifecycle
//!
//! The application lifecycle is represented by the [`App`] type. It is responsible for describing
//! how events sent by the operating system should be handled, and when to stop the application.
//!
//! Rather than having a single central "Event" type, this trait defines multiple methods that
//! will be called according to the event type.
//!
//! # Examples
//!
//! Open a simple window and print a message when the user clicks on it:
//!
//! ```no_run
//! use sage_platform::app::{App, Ctx, Tick, Config};
//! use sage_platform::device::{MouseId, MouseButton};
//!
//! struct MyApp {
//!     close_requested: bool,
//! }
//!
//! impl App for MyApp {
//!     fn close_request(&mut self, ctx: &Ctx) {
//!         self.close_requested = true;
//!     }
//!
//!     fn mouse_button(&mut self, ctx: &Ctx, _mouse: MouseId, button: MouseButton, now_pressed: bool) {
//!         if button == MouseButton::Left && now_pressed {
//!             println!("Hello, World!");
//!         }
//!     }
//!
//!     fn tick(&mut self, ctx: &Ctx) -> Tick {
//!         if self.close_requested {
//!             Tick::Break
//!         } else {
//!             Tick::Continue
//!         }
//!     }
//! }
//!
//! sage_platform::run::<MyApp>(&Config::default());
//! ```

pub mod device;
