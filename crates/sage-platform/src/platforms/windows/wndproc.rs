use std::any::Any;
use std::panic::AssertUnwindSafe;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
use windows_sys::Win32::UI::WindowsAndMessaging::{WM_CLOSE, WM_DESTROY};

use crate::app::App;

use super::Ctx;

/// The signature of a WNDPROC callback.
pub type WndprocFn = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// Calls the default window procedure, but checks that the message does not break any invariants
/// of the [`Ctx`] type. If it does, the function returns 0.
///
/// # Safety
///
/// This function has the same requirements as [`DefWindowProcW`].
unsafe fn checked_default_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if matches!(msg, WM_CLOSE | WM_DESTROY) {
        0
    } else {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

/// The state stored in the **GWLP_USERDATA** pointer of a window.
pub struct State<A> {
    /// When the application panics, the panic payload is stored in this field to be later
    /// resumed out of the wndproc callback.
    payload: Option<Box<dyn Any + Send + 'static>>,
    /// An opaque pointer to an [`App`] implementation.
    app: A,
}

impl<A> State<A> {
    /// Creates a new [`State<A>`] instance.
    #[inline(always)]
    pub const fn new(app: A) -> Self {
        Self { payload: None, app }
    }

    /// Returns an exclusive reference to the [`App`] implementation.
    #[inline(always)]
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// If the payload has been populated, this function resumes the panic.
    ///
    /// # Panics
    ///
    /// That seems pretty obvious.
    pub fn resume_unwind(&mut self) {
        if let Some(payload) = self.payload.take() {
            std::panic::resume_unwind(payload);
        }
    }

    /// The raw WNDPROC callback that will be passed to the Windows API.
    ///
    /// Note that this function is *never* allowed to panic, as that would cause the application to
    /// unwind through foreign code, which is undefined behavior.
    ///
    /// # Safety
    ///
    /// This function assumes that many invariants are upheld.
    ///
    /// * The `hwnd` parameter must be a valid window handle.
    pub unsafe extern "system" fn raw_wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT
    where
        A: App,
    {
        let userdata = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

        if userdata == 0 {
            // The window has not been created yet, so we can't really do anthing. Let's just call
            // the default (but checked) default procedure.

            // SAFETY:
            //  We literally are a window procedure.
            return unsafe { checked_default_window_proc(hwnd, msg, wparam, lparam) };
        }

        // SAFETY:
        //  This is part of the requirements of the `raw_wndproc` function. And this function is
        //  called by the Windows API, so it's not like we can do anything about it.
        //
        //  We know that **GWLP_USERDATA** contains a valid pointer to us because, well, that's the
        //  thing we just got back.
        let ctx = crate::app::Ctx::Windows(unsafe { Ctx::new(hwnd) });

        let state = unsafe { &mut *(userdata as *mut State<A>) };

        // This function may panic. If it does, we need to store the panic payload in the state
        // so that it can be resumed later, but outside of the window procedure.
        let panicky = || match msg {
            WM_CLOSE => {
                state.app.close_request(&ctx);
                0
            }
            _ => unsafe { checked_default_window_proc(hwnd, msg, wparam, lparam) },
        };

        match std::panic::catch_unwind(AssertUnwindSafe(panicky)) {
            Ok(code) => code,
            Err(payload) => {
                state.payload = Some(payload);
                0
            }
        }
    }
}
