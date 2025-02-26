use crate::app::App;

/// Types that can be created from the content of an [`App`] instance.
pub trait FromApp {
    /// Creates a new instance of the type using the state stored in the provided [`App`] instance.
    fn from_app(app: &mut App) -> Self;
}

impl<T: Default> FromApp for T {
    /// Returns the default instance of the type using [`Default::default`].
    #[inline]
    fn from_app(_app: &mut App) -> Self {
        T::default()
    }
}
