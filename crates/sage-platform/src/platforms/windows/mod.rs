//! Defines windows-specific types and functions to create and manage a single-window application.

mod error;
mod window;

pub use error::*;
pub use window::*;
use windows_sys::Win32::Foundation::WIN32_ERROR;

use crate::app::{App, Config, Ctx, RunError, Tick};

use self::owned_window::OwnedWindow;

mod owned_window;
mod wndproc;

/// Starts an application on the Windows platform.
///
/// # Panics
///
/// This function panics if `config.title` contains a null character.
pub fn run<A: App>(args: A::Args, config: &Config) -> Result<A::Output, RunError<A::Error, Error>> {
    let mut window =
        OwnedWindow::new(config, wndproc::State::<A>::raw_wndproc).map_err(RunError::Platform)?;

    let mut state = {
        let ctx = Ctx::Windows(window.as_window());
        wndproc::State::new(A::create(args, &ctx).map_err(RunError::App)?)
    };

    window.set_userdata(&mut state as *mut _ as _);

    loop {
        // Exhaust the event queue.
        while window.peek_message() {
            state.resume_unwind();
        }

        match state.app_mut().tick(&Ctx::Windows(window.as_window())) {
            Tick::Stop(output) => return Ok(output),
            Tick::Block => {
                // Wait until a new message is available.
                window.get_message().map_err(RunError::Platform)?;
                state.resume_unwind();
            }
            Tick::Poll => (),
        }
    }
}

/// Returns the calling thread's last error code.
#[inline(always)]
fn last_error_code() -> WIN32_ERROR {
    use windows_sys::Win32::Foundation::GetLastError;

    // SAFETY:
    //  This is always safe.
    unsafe { GetLastError() }
}
