//! The main game framework used by exported games and the editor.
//!
//! Use this over the fully-featured editor when you need a lightweight
//! framework for your game.

const sage_core = @import("sage_core");
const sage_window = @import("sage_window");
const sage_input = @import("sage_input");

pub const Engine = sage_core.Engine;
pub const Uuid = sage_core.Uuid;
pub const Entity = sage_core.Entity;

pub const run = sage_window.run;
