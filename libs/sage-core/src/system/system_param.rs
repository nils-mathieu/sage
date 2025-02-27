use {
    crate::{
        app::{App, AppCell, Global, missing_global},
        entities::EntityIdAllocator,
        system::SystemAccess,
    },
    std::ops::{Deref, DerefMut},
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

    /// Initializes the system param's state and registers its required access to the application's
    /// resources.
    ///
    /// # Panics
    ///
    /// This function panics if any of the resources required by this system are already accessed
    /// according to the provided [`SystemAccess`].
    ///
    /// # Returns
    ///
    /// This function returns the system param's persistent state.
    fn initialize(app: &mut App, access: &mut SystemAccess) -> Self::State;

    /// Applies any deferred changes to the application
    ///
    /// # Safety
    ///
    /// 1. The provided `state` must have been previously initialized with [`initialize`].
    ///
    /// 2. The provided [`App`] must be the one associated with this system parameter.
    ///
    /// [`initialize`]: SystemParam::initialize
    unsafe fn apply_deferred(state: &mut Self::State, app: &mut App);

    /// Fetches the parameter's state from the application.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// 1. The provided [`App`] is safe to use with the [`SystemParam`]'s accessed resources.
    ///
    /// 2. The provided state has been previously initialized with [`initialize`].
    ///
    /// [`initialize`]: SystemParam::initialize
    unsafe fn fetch<'w>(state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w>;
}

macro_rules! tuple_impl {
    ($($name:ident)*) => {
        #[allow(unused_variables, clippy::unused_unit, unused_unsafe, non_snake_case)]
        unsafe impl<$($name,)*> SystemParam for ($($name,)*)
        where
            $($name: SystemParam,)*
        {
            type State = ($($name::State,)*);
            type Item<'w> = ($($name::Item<'w>,)*);

            fn initialize(app: &mut App, access: &mut SystemAccess) -> Self::State {
                ($($name::initialize(app, access),)*)
            }

            unsafe fn fetch<'w>(state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
                let ($($name,)*) = state;
                unsafe { ($($name::fetch($name, app),)*) }
            }

            unsafe fn apply_deferred(state: &mut Self::State, app: &mut App) {
                let ($($name,)*) = state;
                unsafe { $($name::apply_deferred($name, app);)* }
            }
        }
    }
}

tuple_impl!();
tuple_impl!(A);
tuple_impl!(A B);
tuple_impl!(A B C);
tuple_impl!(A B C D);
tuple_impl!(A B C D E);
tuple_impl!(A B C D E F);
tuple_impl!(A B C D E F G);
tuple_impl!(A B C D E F G H);

unsafe impl SystemParam for &'_ App {
    type State = ();
    type Item<'w> = &'w App;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        unsafe { app.get_ref() }
    }

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}
}

unsafe impl SystemParam for &'_ EntityIdAllocator {
    type State = ();
    type Item<'w> = &'w EntityIdAllocator;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        unsafe { app.get_ref().entities().id_allocator() }
    }
}

/// A global resource.
///
/// `T` may be either `&mut G` or `&G` depending on whether the global resource is accessed
/// mutably or immutably.
pub struct Glob<T>(pub T);

impl<T> Deref for Glob<&'_ T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> Deref for Glob<&'_ mut T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> DerefMut for Glob<&'_ mut T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

unsafe impl<G: Global> SystemParam for Glob<&'_ G> {
    type State = ();
    type Item<'w> = Glob<&'w G>;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        let ret = unsafe {
            app.global()
                .unwrap_or_else(|| missing_global(G::DEBUG_NAME))
        };

        Glob(ret)
    }
}

unsafe impl<G: Global> SystemParam for Glob<&'_ mut G> {
    type State = ();
    type Item<'w> = Glob<&'w mut G>;

    fn initialize(_app: &mut App, _access: &mut SystemAccess) -> Self::State {}

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    #[inline]
    unsafe fn fetch<'w>(_state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        let ret = unsafe {
            app.global_mut()
                .unwrap_or_else(|| missing_global(G::DEBUG_NAME))
        };

        Glob(ret)
    }
}
