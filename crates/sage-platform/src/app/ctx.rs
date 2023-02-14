/// Provides a way for an application to interact with the underlying operating system or window
/// manager.
#[allow(missing_docs)]
pub enum Ctx<'a> {
    #[cfg(target_os = "windows")]
    Windows(crate::windows::Ctx<'a>),
}

#[cfg(feature = "raw-window-handle")]
unsafe impl raw_window_handle::HasRawWindowHandle for Ctx<'_> {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(ctx) => ctx.raw_window_handle(),
        }
    }
}

#[cfg(feature = "raw-window-handle")]
unsafe impl raw_window_handle::HasRawDisplayHandle for Ctx<'_> {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(ctx) => ctx.raw_display_handle(),
        }
    }
}
