use {
    crate::{
        Uuid,
        app::{App, AppCell},
        entities::{
            ArchetypeId, ArchetypeStorage, Component, EntityId, EntityIdAllocator, EntityIndex,
            EntityLocation,
        },
        system::{SystemAccess, SystemParam},
    },
    std::ops::{Deref, DerefMut},
};

/// Contains cached state for a [`Query`] instance allowing efficient iteration
/// over a particular [`App`].
pub struct QueryState<P: QueryParam> {
    /// The [`QueryParam::State`] associated with the query.
    param_state: P::State,
    /// The list of archetypes that match the query's filter.
    matched_archetypes: Vec<ArchetypeId>,
    /// The ID of the largest checked archetype ID.
    ///
    /// This is used when new archetypes are added to the application state and the query
    /// needs to eventually take them into account.
    largest_checked_archetype_id: ArchetypeId,
    /// The filter that the query uses to match archetypes.
    filter: QueryFilter,
}

impl<P: QueryParam> QueryState<P> {
    /// Creates a new [`QueryState<P>`] instance for the provided [`App`].
    pub fn new(app: &mut App, access: &mut SystemAccess) -> Self {
        let mut access = QueryAccess {
            system_access: access,
            filter: QueryFilter::default(),
        };

        Self {
            matched_archetypes: Vec::default(),
            param_state: P::initialize(app, &mut access),
            largest_checked_archetype_id: 0,
            filter: access.filter,
        }
    }

    /// Updates the list of archetypes that match the query's filter.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`App`] is the same one as the one
    /// used to create the [`QueryState<P>`] instance.
    #[inline]
    pub unsafe fn update_matched_archetypes(&mut self, app: &App) {
        if app.entities().archetype_storages().len() > self.largest_checked_archetype_id {
            self.update_matched_archetypes_cold(app);
        }
    }

    #[cold]
    fn update_matched_archetypes_cold(&mut self, app: &App) {
        let new_max_id = app.entities().archetype_storages().len();

        for archetype_id in self.largest_checked_archetype_id..new_max_id {
            let archetype = unsafe {
                app.entities()
                    .archetype_storages()
                    .get_unchecked(archetype_id)
            };

            if self.filter.matches_archetype(archetype) {
                self.matched_archetypes.push(archetype_id);
            }
        }

        self.largest_checked_archetype_id = new_max_id;
    }

    /// Returns the number of matches for the query.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`App`] is the same one as the one
    /// used to create the [`QueryState<P>`] instance.
    pub unsafe fn matched_count(&self, app: AppCell) -> usize {
        self.matched_archetypes
            .iter()
            .map(|&archetype| unsafe {
                app.get_ref()
                    .entities()
                    .archetype_storages()
                    .get_unchecked(archetype)
                    .len()
            })
            .sum()
    }

    /// Creates a [`Query<P>`] instance that uses this [`QueryState<P>`] to allow
    /// access to the entities of the provided application state.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that the component accesses requested by the query are available
    ///   in the provided application state.
    ///
    /// - The caller must ensure that the provided application state is the same one as the one
    ///   used to create the [`QueryState<P>`] instance.
    #[inline]
    pub unsafe fn make_query<'w>(&'w self, app: AppCell<'w>) -> Query<'w, P> {
        Query { app, state: self }
    }

    /// Turns this [`QueryState<P>`] into a consuming [`QueryIntoIter<P>`] instance.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// 1. The provided application state is the same one as the one used to create the
    ///    [`QueryState<P>`] instance.
    ///
    /// 2. The component accesses requested by the query are available in the provided
    ///    application state.
    #[inline]
    pub unsafe fn into_iter(self, app: AppCell) -> QueryIntoIter<P> {
        let iter_state = unsafe { P::create_iter_state(&self.param_state, app) };

        QueryIntoIter {
            state: self.param_state,
            iter_state,
            archetypes: unsafe { app.get_ref().entities().archetype_storages() },
            archetype_ids: self.matched_archetypes.into_iter(),
            range: 0..0,
        }
    }
}

/// A system parameter that allows accessing all entities with a specific set of
/// components (according to the query's filter and fetch generic parameters).
pub struct Query<'w, P: QueryParam> {
    /// All archetypes in the state.
    ///
    /// All requested resources must be available.
    app: AppCell<'w>,
    /// The state of the query.
    state: &'w QueryState<P>,
}

impl<'w, P: QueryParam> Query<'w, P> {
    /// Creates a new iterator that returns the entities that match the query's filter.
    pub fn iter(&mut self) -> QueryIter<'w, P> {
        QueryIter {
            state: &self.state.param_state,
            iter_state: unsafe { P::create_iter_state(&self.state.param_state, self.app) },
            archetypes: unsafe { self.app.get_ref().entities().archetype_storages() },
            archetype_ids: self.state.matched_archetypes.iter(),
            range: 0..0,
        }
    }
}

unsafe impl<P> SystemParam for Query<'_, P>
where
    P: 'static + QueryParam,
{
    type Item<'w> = Query<'w, P>;
    type State = QueryState<P>;

    fn initialize(app: &mut App, access: &mut SystemAccess) -> Self::State {
        let mut state = QueryState::new(app, access);
        unsafe { state.update_matched_archetypes(app) };
        state
    }

    unsafe fn apply_deferred(_state: &mut Self::State, _app: &mut App) {}

    #[inline]
    unsafe fn fetch<'w>(state: &'w mut Self::State, app: AppCell<'w>) -> Self::Item<'w> {
        unsafe { state.make_query(app) }
    }
}

/// An [`Iterator`] over the entities that a query matches.
pub struct QueryIter<'w, P: QueryParam> {
    state: &'w P::State,
    iter_state: P::IterState<'w>,
    archetypes: &'w [ArchetypeStorage],
    archetype_ids: std::slice::Iter<'w, ArchetypeId>,
    range: std::ops::Range<usize>,
}

impl<'w, P: QueryParam> Iterator for QueryIter<'w, P> {
    type Item = P::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.range.next() {
                Some(index) => {
                    // SAFETY: We're keeping all invariants in check.
                    unsafe {
                        break Some(P::fetch(self.state, &mut self.iter_state, index));
                    }
                }
                None => {
                    let archetype_id = *self.archetype_ids.next()?;

                    // SAFETY: The archetype IDs stored in the state are always valid.
                    let storage = unsafe { self.archetypes.get_unchecked(archetype_id) };

                    // SAFETY: We're keeping all invariants in check.
                    unsafe {
                        P::set_archetype_storage(self.state, &mut self.iter_state, storage);
                    }

                    self.range = 0..storage.len();
                }
            }
        }
    }
}

/// An iterator that consumes a [`Query`] and returns the entities that match the query's filter.
pub struct QueryIntoIter<'w, P: QueryParam> {
    state: P::State,
    iter_state: P::IterState<'w>,
    archetypes: &'w [ArchetypeStorage],
    archetype_ids: std::vec::IntoIter<ArchetypeId>,
    range: std::ops::Range<usize>,
}

impl<'w, P: QueryParam> Iterator for QueryIntoIter<'w, P> {
    type Item = P::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.range.next() {
                Some(index) => {
                    // SAFETY: We're keeping all invariants in check.
                    unsafe {
                        break Some(P::fetch(&self.state, &mut self.iter_state, index));
                    }
                }
                None => {
                    let archetype_id = self.archetype_ids.next()?;

                    // SAFETY: The archetype IDs stored in the state are always valid.
                    let storage = unsafe { self.archetypes.get_unchecked(archetype_id) };

                    // SAFETY: We're keeping all invariants in check.
                    unsafe {
                        P::set_archetype_storage(&self.state, &mut self.iter_state, storage);
                    }

                    self.range = 0..storage.len();
                }
            }
        }
    }
}

type Set<T> = hashbrown::HashSet<T, foldhash::fast::FixedState>;

/// The filter that a query uses to match entities.
#[derive(Default, Debug)]
pub struct QueryFilter {
    /// The components that the query wants to match.
    ///
    /// Components present here are guaranteed to be present in all entities that the query
    /// matches.
    pub with: Set<Uuid>,
    /// The components that the query wants to exclude.
    ///
    /// Components present here are guaranteed to be absent in all entities that the query matches.
    pub without: Set<Uuid>,
}

impl QueryFilter {
    /// Returns whether the filter matches the provided archetype.
    pub fn matches_archetype(&self, archetype: &ArchetypeStorage) -> bool {
        for &with in &self.with {
            if !archetype.has_component(with) {
                return false;
            }
        }

        for &without in &self.without {
            if archetype.has_component(without) {
                return false;
            }
        }

        true
    }
}

/// A structure that holds the query parameters and resources that a query accesses.
pub struct QueryAccess<'a> {
    /// The associated system access.
    pub system_access: &'a mut SystemAccess,
    /// The query filter being built.
    pub filter: QueryFilter,
}

impl QueryAccess<'_> {
    /// Registers a component that the query wants to include.
    pub fn with(&mut self, component: Uuid) {
        self.filter.with.insert(component);
    }

    /// Registers a component that the query wants to exclude.
    pub fn without(&mut self, component: Uuid) {
        self.filter.without.insert(component);
    }
}

impl Deref for QueryAccess<'_> {
    type Target = SystemAccess;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.system_access
    }
}

impl DerefMut for QueryAccess<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.system_access
    }
}

/// A trait for query parameters.
///
/// Query parameters are used to filter and fetch entities from the application state.
///
/// # Safety
///
/// Implementators must ensure that:
///
/// 1. None of the resources accessed by the query parameter conflict with a resource previously
///    registered by another parameter according to the provided [`SystemAccess`].
///
/// 2. The `fetch` method must only access resources whose access has been registered in the
///    [`register_access`] method.
pub unsafe trait QueryParam {
    /// The immutable state of the query parameter.
    type State: Send + Sync + 'static;

    /// The output type of the [`fetch`] method.
    ///
    /// [`fetch`]: QueryParam::fetch
    type Item<'w>;

    /// The mutable state that will be continuously updated while iterating over
    /// the query's matched entities.
    type IterState<'w>;

    /// Creates an instance of the query parameter's state.
    fn initialize(app: &mut App, access: &mut QueryAccess) -> Self::State;

    /// Creates a new iterator state.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`AppCell`] is the same one as the one used to
    /// create the query parameter's state.
    ///
    /// It must provide access to all resources required by the query parameter.
    unsafe fn create_iter_state<'w>(state: &Self::State, app: AppCell<'w>) -> Self::IterState<'w>;

    /// Updates the provided iterator state for a new archetype storage.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided storage contains the components requested
    /// by the query's registered accesses.
    unsafe fn set_archetype_storage<'w>(
        state: &Self::State,
        iter: &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    );

    /// Fetches the query parameter's state from the application.
    ///
    /// # Parameters
    ///
    /// - `state`: The state of the query parameter.
    ///
    /// - `iter`: The state required to iterate over the current archetype storage's content.
    ///
    /// - `index`: The index of the entity to query in the current archetype storage.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// 1. The provided index is within the bounds of the current archetype storage.
    ///
    /// 2. The `iter` state must come from a previous call to [`create_iter_state`]
    ///    for which the input storage is still valid for the access requested by the
    ///    [`register_access`] method.
    ///
    /// 3. The provided `state` must have been previously initialized with [`create_state`]
    ///    previously using the same [`App`].
    ///
    /// [`create_iter_state`]: QueryParam::create_iter_state
    /// [`register_access`]: QueryParam::register_access
    /// [`create_state`]: QueryParam::create_state
    unsafe fn fetch<'w>(
        state: &Self::State,
        iter: &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w>;
}

/// Like [`QueryParam`], but with the additional requirement that the query parameter
/// must only *read* from the application state.
///
/// # Safety
///
/// Implementors of this trait must ensure that the query parameter only reads from the
/// application state.
pub unsafe trait ReadOnlyQueryParam: QueryParam {}

unsafe impl QueryParam for EntityId {
    type State = ();
    type Item<'w> = EntityId;
    type IterState<'w> = (*const EntityIndex, &'w EntityIdAllocator<EntityLocation>);

    fn initialize(_app: &mut App, _access: &mut QueryAccess) -> Self::State {}

    unsafe fn create_iter_state<'w>(_state: &Self::State, app: AppCell<'w>) -> Self::IterState<'w> {
        let id_allocator = unsafe { app.get_ref().entities().id_allocator() };
        (std::ptr::null(), id_allocator)
    }

    unsafe fn set_archetype_storage<'w>(
        _state: &Self::State,
        (indices, _): &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    ) {
        *indices = storage.entity_indices().as_ptr();
    }

    unsafe fn fetch<'w>(
        _state: &Self::State,
        (indices, ids): &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w> {
        // SAFETY: The caller must provide a valid index.
        let entity_index = unsafe { *indices.add(index) };

        // SAFETY: The entity indices stored in an archetype storage are always valid.
        unsafe { ids.get_id_for_index_unchecked(entity_index) }
    }
}

unsafe impl ReadOnlyQueryParam for EntityId {}

unsafe impl<T: Component> QueryParam for &'_ T {
    type State = ();
    type Item<'w> = &'w T;
    type IterState<'w> = *const T;

    fn initialize(_app: &mut App, access: &mut QueryAccess) -> Self::State {
        access.with(T::UUID);
        access.system_access.read_component(T::UUID);
    }

    #[inline]
    unsafe fn create_iter_state<'w>(
        _state: &Self::State,
        _app: AppCell<'w>,
    ) -> Self::IterState<'w> {
        std::ptr::null()
    }

    unsafe fn set_archetype_storage<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    ) {
        *iter = unsafe {
            storage
                .get_column(T::UUID)
                .unwrap_unchecked()
                .as_ptr()
                .as_ptr::<T>()
        };
    }

    #[inline]
    unsafe fn fetch<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w> {
        unsafe { &*iter.add(index) }
    }
}

unsafe impl<T: Component> ReadOnlyQueryParam for &'_ T {}

unsafe impl<T: Component> QueryParam for &'_ mut T {
    type State = ();
    type Item<'w> = &'w mut T;
    type IterState<'w> = *mut T;

    fn initialize(_app: &mut App, access: &mut QueryAccess) -> Self::State {
        access.with(T::UUID);
        access.write_component(T::UUID);
    }

    #[inline]
    unsafe fn create_iter_state<'w>(
        _state: &Self::State,
        _app: AppCell<'w>,
    ) -> Self::IterState<'w> {
        std::ptr::null_mut()
    }

    unsafe fn set_archetype_storage<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    ) {
        *iter = unsafe {
            storage
                .get_column(T::UUID)
                .unwrap_unchecked()
                .as_ptr()
                .as_ptr::<T>()
        };
    }

    #[inline]
    unsafe fn fetch<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w> {
        unsafe { &mut *iter.add(index) }
    }
}

unsafe impl<T: Component> QueryParam for Option<&'_ T> {
    type State = ();
    type IterState<'w> = *const T;
    type Item<'w> = Option<&'w T>;

    fn initialize(_app: &mut App, access: &mut QueryAccess) -> Self::State {
        access.read_component(T::UUID);
    }

    unsafe fn create_iter_state<'w>(
        _state: &Self::State,
        _app: AppCell<'w>,
    ) -> Self::IterState<'w> {
        std::ptr::null()
    }

    unsafe fn set_archetype_storage<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    ) {
        *iter = storage
            .get_column(T::UUID)
            .map(|x| x.as_ptr().as_ptr::<T>() as *const T)
            .unwrap_or(std::ptr::null())
    }

    unsafe fn fetch<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w> {
        if iter.is_null() {
            None
        } else {
            unsafe { Some(&*iter.add(index)) }
        }
    }
}

unsafe impl<T: Component> ReadOnlyQueryParam for Option<&'_ T> {}

unsafe impl<T: Component> QueryParam for Option<&'_ mut T> {
    type State = ();
    type IterState<'w> = *mut T;
    type Item<'w> = Option<&'w mut T>;

    fn initialize(_app: &mut App, access: &mut QueryAccess) -> Self::State {
        access.write_component(T::UUID);
    }

    #[inline]
    unsafe fn create_iter_state<'w>(
        _state: &Self::State,
        _app: AppCell<'w>,
    ) -> Self::IterState<'w> {
        std::ptr::null_mut()
    }

    unsafe fn set_archetype_storage<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        storage: &'w ArchetypeStorage,
    ) {
        *iter = storage
            .get_column(T::UUID)
            .map(|x| x.as_ptr().as_ptr::<T>())
            .unwrap_or(std::ptr::null_mut())
    }

    unsafe fn fetch<'w>(
        _state: &Self::State,
        iter: &mut Self::IterState<'w>,
        index: usize,
    ) -> Self::Item<'w> {
        if iter.is_null() {
            None
        } else {
            unsafe { Some(&mut *iter.add(index)) }
        }
    }
}

macro_rules! tuple_impl {
    ($($name:ident $name2:ident),*) => {
        #[allow(unused_variables, clippy::unused_unit, non_snake_case, unused_unsafe)]
        unsafe impl<$($name,)*> QueryParam for ($($name,)*)
        where
            $($name: QueryParam,)*
        {
            type State = ($($name::State,)*);
            type Item<'w> = ($($name::Item<'w>,)*);
            type IterState<'w> = ($($name::IterState<'w>,)*);

            fn initialize(app: &mut App, access: &mut QueryAccess) -> Self::State {
                ($($name::initialize(app, access),)*)
            }

            unsafe fn create_iter_state<'w>(state: &Self::State, app:  AppCell<'w>) -> Self::IterState<'w> {
                let ($($name,)*) = state;
                unsafe { ($($name::create_iter_state($name, app),)*) }
            }

            unsafe fn set_archetype_storage<'w>(
                state: &Self::State,
                iter: &mut Self::IterState<'w>,
                storage: &'w ArchetypeStorage,
            ) {
                let ($($name,)*) = state;
                let ($($name2,)*) = iter;
                unsafe { $(<$name as QueryParam>::set_archetype_storage($name, $name2, storage);)* }
            }

            unsafe fn fetch<'w>(
                state: &Self::State,
                iter: &mut Self::IterState<'w>,
                index: usize,
            ) -> Self::Item<'w> {
                let ($($name,)*) = state;
                let ($($name2,)*) = iter;
                unsafe { ($(<$name as QueryParam>::fetch($name, $name2, index),)*) }
            }
        }

        unsafe impl<$($name,)*> ReadOnlyQueryParam for ($($name,)*)
        where
            $($name: ReadOnlyQueryParam,)*
        {}
    };
}

tuple_impl!();
tuple_impl!(A a);
tuple_impl!(A a, B b);
tuple_impl!(A a, B b, C c);
tuple_impl!(A a, B b, C c, D d);
tuple_impl!(A a, B b, C c, D d, E e);
tuple_impl!(A a, B b, C c, D d, E e, F f);
tuple_impl!(A a, B b, C c, D d, E e, F f, G g);
tuple_impl!(A a, B b, C c, D d, E e, F f, G g, H h);
