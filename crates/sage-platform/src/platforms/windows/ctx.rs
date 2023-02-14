use std::marker::PhantomData;

use windows_sys::Win32::Foundation::{HINSTANCE, HWND};

/// Represents a live window reference.
///
/// # Lifetimes
///
/// `'wnd` is the lifetime of the window itself. Because the [`Ctx`] type does not actually
/// "owns" its window, it has to make sure that the resources it reference are still valid.
#[derive(Debug)]
pub struct Ctx<'wnd> {
    hwnd: HWND,
    _wnd: PhantomData<&'wnd ()>,
}

impl<'wnd> Ctx<'wnd> {
    /// Creates a new window from a raw `HWND` handle.
    ///
    /// # Safety
    ///
    /// This function assumes that `hwnd` is a valid window handle, and that it will remain valid
    /// for the lifetime `'wnd`. The "referenced" window must be logically borrowed exclusively
    /// for that lifetime.
    #[inline(always)]
    pub(super) const unsafe fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            _wnd: PhantomData,
        }
    }

    /// Returns the window handle.
    ///
    /// # Note On Safety
    ///
    /// This function is safe by itself because it does not allow *safe* code to produce undefined
    /// behavior, but note that modifying the window in a way that breaks the invariants of the
    /// [`Ctx`] type is undefined behavior.
    #[inline(always)]
    pub const fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Returns the [`HINSTANCE`] associated with the window.
    #[inline(always)]
    pub fn hinstance(&self) -> HINSTANCE {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_HINSTANCE};

        unsafe { GetWindowLongPtrW(self.hwnd, GWLP_HINSTANCE) }
    }
}

#[cfg(feature = "raw-window-handle")]
unsafe impl raw_window_handle::HasRawWindowHandle for Ctx<'_> {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut window = raw_window_handle::Win32WindowHandle::empty();
        window.hwnd = self.hwnd as *mut std::ffi::c_void;
        window.hinstance = self.hinstance() as *mut std::ffi::c_void;
        raw_window_handle::RawWindowHandle::Win32(window)
    }
}

#[cfg(feature = "raw-window-handle")]
unsafe impl raw_window_handle::HasRawDisplayHandle for &Ctx<'_> {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        let display = raw_window_handle::WindowsDisplayHandle::empty();
        raw_window_handle::RawDisplayHandle::Windows(display)
    }
}
