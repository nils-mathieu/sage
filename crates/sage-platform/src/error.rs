use std::fmt;

/// An error which might occur when interacting with the underlying platform.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Error {
    #[cfg(target_os = "windows")]
    Windows(crate::windows::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            #[cfg(target_os = "windows")]
            Self::Windows(ref err) => fmt::Display::fmt(err, f),
        }
    }
}
