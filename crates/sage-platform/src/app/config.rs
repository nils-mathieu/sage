/// Describes the initial configuration of an application.
///
/// More specifically, this type is used to describe the window that may eventually be created
/// on most desktop platforms.
///
/// When a platform does not support creating a window, or a specific parameter, the value is
/// ignored. Those should be treated as hints and not requirements.
pub struct Config<'a> {
    /// The title of the window created for the application.
    ///
    /// **Default:** `"Sage Application"`
    pub title: &'a str,
    /// The initial size of the window created for the application.
    ///
    /// If no value is provided, the window will be created with a platform-specific default size.
    ///
    /// **Default:** `None`
    pub size: Option<(u32, u32)>,
    /// The initial position of the window created for the application.
    ///
    /// If no value is provided, the window will be created with a platform-specific default
    /// position.
    ///
    /// **Default:** `None`
    pub position: Option<(i32, i32)>,
    /// Whether the application should support non-opaque rendering.
    ///
    /// This often impact slightly performances, but can be useful for some applications.
    ///
    /// **Default:** `false`
    pub transparent: bool,
    /// Whether the window should be visible from the begining.
    ///
    /// It can be useful to turn this off when you want to render something before showing the
    /// window.
    ///
    /// **Default:** `true`
    pub visible: bool,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Self {
            title: "Sage Application",
            size: None,
            position: None,
            transparent: false,
            visible: true,
        }
    }
}
