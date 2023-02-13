use std::fmt;

/// An error which may occur whilst interacting with the Windows platform.
#[derive(Debug, Clone)]
pub enum Error {
    /// The Windows API behaved unexpectedly.
    UnexpectedBehavior,
    /// The window class was already registered.
    ClassAlreadyRegistered,
    /// The provided configuration is invalid.
    UnsupportedConfig,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnexpectedBehavior => f.write_str("the Windows API behaved unexpectedly"),
            Self::UnsupportedConfig => f.write_str("unsupported window configuration"),
            Self::ClassAlreadyRegistered => {
                f.write_str("the window class `Sage Window` is already registered")
            }
        }
    }
}
