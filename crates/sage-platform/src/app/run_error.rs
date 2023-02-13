use crate::Error;

/// When starting an application up, either the application can fail to create itself (with
/// [`App::create`](super::App::run)), or an error can occur when interacting with the underlying
/// platform.
#[derive(Debug, Clone)]
pub enum RunError<A, P = Error> {
    /// The application failed to create itself.
    App(A),
    /// An error occured when interacting with the platform.
    Platform(P),
}

impl<A, P> RunError<A, P> {
    /// Returns a mapper function that maps a [`RunError<A, P>`] to a [`RunError<A, Q>`].
    pub fn map_platform<Q>(f: impl FnOnce(P) -> Q) -> impl FnOnce(Self) -> RunError<A, Q> {
        move |this| match this {
            Self::App(a) => RunError::App(a),
            Self::Platform(p) => RunError::Platform(f(p)),
        }
    }
}
