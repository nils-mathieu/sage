/// Provides a way for an application to interact with the underlying operating system or window
/// manager.
#[allow(missing_docs)]
pub enum Ctx<'a> {
    #[cfg(target_os = "windows")]
    Windows(crate::windows::Window<'a>),
}
