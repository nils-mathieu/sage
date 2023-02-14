//! Defines windows-specific types and functions to create and manage a single-window application.

mod ctx;
mod error;

pub use ctx::*;
pub use error::*;
use windows_sys::Win32::Foundation::WIN32_ERROR;

use crate::app::{App, Config, RunError, Tick};

use self::owned_window::Window;

mod owned_window;
mod wndproc;

/// A unique identifier for a device.
pub type DeviceId = windows_sys::Win32::Foundation::HANDLE;

/// Starts an application on the Windows platform.
///
/// # Panics
///
/// This function panics if `config.title` contains a null character.
pub fn run<A: App>(args: A::Args, config: &Config) -> Result<A::Output, RunError<A::Error, Error>> {
    let mut window =
        Window::new(config, wndproc::State::<A>::raw_wndproc).map_err(RunError::Platform)?;

    let app = A::create(args, &crate::app::Ctx::Windows(window.as_ctx())).map_err(RunError::App)?;
    let mut state = wndproc::State::new(app);

    window.set_userdata(&mut state as *mut _ as _);

    loop {
        // Exhaust the event queue.
        while window.peek_message() {
            state.resume_unwind();
        }

        match state
            .app_mut()
            .tick(&crate::app::Ctx::Windows(window.as_ctx()))
        {
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
