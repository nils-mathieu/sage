use crate::entities::entity_layout::EntityLayout;
use crate::entities::{ComponentId, Entities, EntityPtr, EntitySlice, Tables};
use crate::{Component, Entity};

/// A trait for types that can be extracted from an [`Entities`] collection.
pub trait Query<'e> {
    /// The state required to efficiently extract the components from an [`Entities`] collection.
    type State;

    /// Creates the state required to efficiently extract the components from an [`Entities`]
    /// collection.
    ///
    /// If the function returns [`None`], then the query does not match the provided
    /// archetype.
    fn init(entities: &'e EntityLayout) -> Option<Self::State>;

    /// Extracts the components from the provided [`Entities`] collection.
    ///
    /// # Safety
    ///
    /// This function must only be called with a state that was created by [`init`],
    /// with the same [`Entities`] collection.
    ///
    /// The provided entity pointer must be valid and it must be safe to access the components
    /// of the entity that it points to in the way that the query requires.
    ///
    /// [`init`]: Self::init
    unsafe fn extract(state: &Self::State, id: Entity, entity: EntityPtr<'e>) -> Self;
}

impl<'e> Query<'e> for Entity {
    type State = ();

    #[inline(always)]
    fn init(_: &'e EntityLayout) -> Option<Self::State> {
        Some(())
    }

    #[inline(always)]
    unsafe fn extract(_state: &Self::State, id: Entity, _entity: EntityPtr<'e>) -> Self {
        id
    }
}

impl<'e, T: Component> Query<'e> for &'e T {
    type State = usize;

    #[inline(always)]
    fn init(layout: &'e EntityLayout) -> Option<Self::State> {
        layout
            .field_of(ComponentId::of::<T>())
            .map(|field| field.offset)
    }

    #[inline(always)]
    unsafe fn extract(state: &Self::State, _id: Entity, entity: EntityPtr<'e>) -> Self {
        &*entity.as_ptr().add(*state).cast::<T>()
    }
}

impl<'e, T: Component> Query<'e> for &'e mut T {
    type State = usize;

    #[inline(always)]
    fn init(layout: &'e EntityLayout) -> Option<Self::State> {
        layout
            .field_of(ComponentId::of::<T>())
            .map(|field| field.offset)
    }

    #[inline(always)]
    unsafe fn extract(state: &Self::State, _id: Entity, entity: EntityPtr<'e>) -> Self {
        &mut *entity.as_ptr().add(*state).cast::<T>()
    }
}

impl<'e, Q: Query<'e>> Query<'e> for Option<Q> {
    type State = Option<<Q as Query<'e>>::State>;

    #[inline(always)]
    fn init(layout: &'e EntityLayout) -> Option<Self::State> {
        Some(Q::init(layout))
    }

    #[inline(always)]
    unsafe fn extract(state: &Self::State, id: Entity, entity: EntityPtr<'e>) -> Self {
        state.as_ref().map(|state| Q::extract(state, id, entity))
    }
}

macro_rules! impl_tuple {
    (has_duplicates $first:expr, $($ty:expr,)*) => {
        $( $first == $ty || )* impl_tuple!(has_duplicates $($ty,)*)
    };

    (has_duplicates) => {
        false
    };

    ($($ty:ident),*) => {
        impl<'e, $($ty: Query<'e>),*> Query<'e> for ($($ty,)*) {
            type State = ($($ty::State,)*);

            #[inline(always)]
            #[allow(unused_variables)]
            fn init(layout: &'e EntityLayout) -> Option<Self::State> {
                Some(($($ty::init(layout)?,)*))
            }

            #[inline(always)]
            #[allow(clippy::unused_unit, unused_variables, non_snake_case)]
            unsafe fn extract(state: &Self::State, id: Entity, entity: EntityPtr<'e>) -> Self {
                let ($($ty,)*) = state;
                ($($ty::extract($ty, id, entity),)*)
            }
        }
    };
}

impl_tuple!();
impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);

/// An iterator over the entities of an entity table.
struct QueryTableIter<'e, Q: Query<'e>> {
    ids: &'e [Entity],
    entities: EntitySlice<'e>,
    state: <Q as Query<'e>>::State,
    index: usize,
}

impl<'e, Q: Query<'e>> Iterator for QueryTableIter<'e, Q> {
    type Item = Q;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entities.get(self.index)?;
        let id = unsafe { *self.ids.get_unchecked(self.index) };
        self.index += 1;
        Some(unsafe { Q::extract(&self.state, id, entity) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.entities.len() - self.index;
        (len, Some(len))
    }
}

/// An iterator over the components of an [`Entities`] collection that match a query.
pub struct QueryIter<'e, Q: Query<'e>> {
    /// The tables that is being iterated over.
    tables: Tables<'e>,
    /// The current table being queried, as well as the state required to extract the query.
    current: Option<QueryTableIter<'e, Q>>,
}

impl<'e, Q: Query<'e>> QueryIter<'e, Q> {
    /// Creates a new iterator over the provided [`Entities`] collection.
    ///
    /// # Safety
    ///
    /// It must be safe to access the entities of [`Entities`] in the way requested by `Q`.
    pub unsafe fn new_unchecked(entities: &'e Entities) -> Self {
        Self {
            tables: entities.tables(),
            current: None,
        }
    }
}

impl<'e, Q: Query<'e>> Iterator for QueryIter<'e, Q> {
    type Item = Q;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.current {
                Some(ref mut iter) => match iter.next() {
                    Some(item) => return Some(item),
                    None => self.current = None,
                },
                None => {
                    let (ids, entities) = self.tables.next()?;
                    let state = Q::init(entities.layout())?;
                    self.current = Some(QueryTableIter {
                        ids,
                        entities,
                        state,
                        index: 0,
                    });
                }
            }
        }
    }
}
