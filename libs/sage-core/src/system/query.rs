use crate::{
    app::App,
    entities::{
        ArchetypeId, ArchetypeStorage, Component, EntityId, EntityIdAllocator, EntityIndex,
        EntityLocation,
    },
    system::{SystemAccess, SystemParam},
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
}

impl<P: QueryParam> QueryState<P> {
    /// Creates a new [`QueryState<P>`] instance for the provided [`App`].
    pub fn new(app: &mut App) -> Self {
        let param_state = P::create_state(app);

        Self {
            matched_archetypes: Vec::default(),
            param_state,
            largest_checked_archetype_id: 0,
        }
    }

    /// Updates the list of archetypes that match the query's filter.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`App`] is the same one as the one
    /// used to create the [`QueryState<P>`] instance.
    pub unsafe fn update_matched_archetypes(&mut self, app: &App) {
        let new_max_id = app.entities().archetype_storages().len();

        for _id in self.largest_checked_archetype_id..new_max_id {}

        self.largest_checked_archetype_id = new_max_id;
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
    pub unsafe fn make_query_unchecked<'w>(&'w self, app: &'w App) -> Query<'w, P> {
        Query { app, state: self }
    }
}

/// A system parameter that allows accessing all entities with a specific set of
/// components (according to the query's filter and fetch generic parameters).
pub struct Query<'w, P: QueryParam> {
    /// All archetypes in the state.
    ///
    /// All requested resources must be available.
    app: &'w App,
    /// The state of the query.
    state: &'w QueryState<P>,
}

impl<'w, P: QueryParam> Query<'w, P> {
    /// Creates a new iterator that returns the entities that match the query's filter.
    pub fn iter(&mut self, archetypes: &'w [ArchetypeStorage]) -> QueryIter<'w, P> {
        QueryIter {
            state: self.state,
            iter_state: P::create_iter_state(&self.state.param_state, self.app),
            archetypes,
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

    #[inline]
    fn register_access(access: &mut SystemAccess) {
        P::register_access(access);
    }

    #[inline]
    fn create_state(app: &mut App) -> Self::State {
        QueryState::new(app)
    }

    #[inline]
    unsafe fn fetch<'w>(state: &'w Self::State, app: &'w App) -> Self::Item<'w> {
        unsafe { state.make_query_unchecked(app) }
    }
}

/// An [`Iterator`] over the entities that a query matches.
pub struct QueryIter<'w, P: QueryParam> {
    state: &'w QueryState<P>,
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
                        break Some(P::fetch(
                            &self.state.param_state,
                            &mut self.iter_state,
                            index,
                        ));
                    }
                }
                None => {
                    let archetype_id = *self.archetype_ids.next()?;

                    // SAFETY: The archetype IDs stored in the state are always valid.
                    let storage = unsafe { self.archetypes.get_unchecked(archetype_id) };

                    // SAFETY: We're keeping all invariants in check.
                    unsafe {
                        P::set_archetype_storage(
                            &self.state.param_state,
                            &mut self.iter_state,
                            storage,
                        );
                    }

                    self.range = 0..storage.len();
                }
            }
        }
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

    /// Registers the resources that the query parameter requires to construct itself.
    fn register_access(access: &mut SystemAccess);

    /// Creates an instance of the query parameter's state.
    fn create_state(app: &mut App) -> Self::State;

    /// Creates a new iterator state.
    fn create_iter_state<'w>(state: &Self::State, app: &'w App) -> Self::IterState<'w>;

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

unsafe impl QueryParam for () {
    type State = ();
    type Item<'w> = ();
    type IterState<'w> = ();

    fn create_state(_app: &mut App) -> Self::State {}
    fn create_iter_state(_state: &Self::State, _app: &App) {}
    unsafe fn set_archetype_storage(
        _state: &Self::State,
        _iter: &mut Self::IterState<'_>,
        _storage: &ArchetypeStorage,
    ) {
    }
    unsafe fn fetch<'w>(
        _state: &Self::State,
        _iter: &mut Self::IterState<'w>,
        _index: usize,
    ) -> Self::Item<'w> {
    }
    fn register_access(_access: &mut SystemAccess) {}
}

unsafe impl QueryParam for EntityId {
    type State = ();
    type Item<'w> = EntityId;
    type IterState<'w> = (*const EntityIndex, &'w EntityIdAllocator<EntityLocation>);

    fn create_state(_app: &mut App) -> Self::State {}

    fn create_iter_state<'w>(_state: &Self::State, app: &'w App) -> Self::IterState<'w> {
        (std::ptr::null(), app.entities().id_allocator())
    }

    fn register_access(_access: &mut SystemAccess) {}

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

unsafe impl<T: Component> QueryParam for &'_ T {
    type State = ();
    type Item<'w> = &'w T;
    type IterState<'w> = *const T;

    fn create_state(_app: &mut App) -> Self::State {}

    #[inline]
    fn create_iter_state<'w>(_state: &Self::State, _app: &'w App) -> Self::IterState<'w> {
        std::ptr::null()
    }

    fn register_access(_access: &mut SystemAccess) {}

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

unsafe impl<T: Component> QueryParam for &'_ mut T {
    type State = ();
    type Item<'w> = &'w mut T;
    type IterState<'w> = *mut T;

    fn create_state(_app: &mut App) -> Self::State {}

    #[inline]
    fn create_iter_state<'w>(_state: &Self::State, _app: &'w App) -> Self::IterState<'w> {
        std::ptr::null_mut()
    }

    fn register_access(_access: &mut SystemAccess) {}

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
