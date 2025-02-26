use crate::{
    app::{App, Global, missing_global},
    system::SystemAccess,
};

/// A trait for system parameters.
///
/// # Safety
///
/// Implementators must ensure that:
///
/// 1. None of the resources accessed by the system conflict with a resource previously
///    registered by another system.
///
/// 2. The `fetch` method must only access resources whose access has been registered in the
///    [`register_access`] method.
///
/// [`register_access`]: SystemParam::register_access
pub unsafe trait SystemParam {
    /// Some state that is kept between system invocations.
    type State: Send + Sync + 'static;

    /// The output item of the parameter.
    type Item<'w>;

    /// Registers the resources that the parameter requires to construct itself.
    ///
    /// # Panics
    ///
    /// This function panics if any of the resources required by this system are already accessed
    /// according to the provided [`SystemAccess`].
    fn register_access(access: &mut SystemAccess);

    /// Initializes the parameter's state.
    fn create_state(app: &mut App) -> Self::State;

    /// Fetches the parameter's state from the application.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// 1. The provided [`App`] is safe to use with the [`SystemParam`]'s accessed resources.
    ///
    /// 2. The provided state has been previously initialized with [`create_state`].
    ///
    /// [`create_state`]: SystemParam::create_state
    unsafe fn fetch<'w>(state: &'w Self::State, app: &'w App) -> Self::Item<'w>;
}

unsafe impl SystemParam for () {
    type State = ();
    type Item<'w> = ();

    fn register_access(_access: &mut SystemAccess) {}

    fn create_state(_state: &mut App) -> Self::State {}

    unsafe fn fetch<'w>(_state: &'w Self::State, _app: &'w App) -> Self::Item<'w> {}
}

/// A global resource.
///
/// `T` may be either `&mut G` or `&G` depending on whether the global resource is accessed
/// mutably or immutably.
pub struct Glob<T>(pub T);

unsafe impl<G: Global> SystemParam for Glob<&'_ G> {
    type State = ();
    type Item<'w> = &'w G;

    fn register_access(_access: &mut SystemAccess) {}

    fn create_state(_state: &mut App) -> Self::State {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w Self::State, app: &'w App) -> Self::Item<'w> {
        app.global()
    }
}

unsafe impl<G: Global> SystemParam for Glob<&'_ mut G> {
    type State = ();
    type Item<'w> = &'w mut G;

    fn register_access(_access: &mut SystemAccess) {}

    fn create_state(_state: &mut App) -> Self::State {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w Self::State, app: &'w App) -> Self::Item<'w> {
        let p = app
            .globals()
            .get_raw(G::UUID)
            .unwrap_or_else(|| missing_global(G::DEBUG_NAME))
            .data();

        // SAFETY: The caller must ensure that access to the global is safe.
        unsafe { p.as_mut() }
    }
}
