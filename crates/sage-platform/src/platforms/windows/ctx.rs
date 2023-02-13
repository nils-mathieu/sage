use std::marker::PhantomData;

use windows_sys::Win32::Foundation::HWND;

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
}
