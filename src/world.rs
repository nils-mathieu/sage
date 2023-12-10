use crate::entities::add_components::AddComponents;
use crate::entities::entity_layout::{Components, StaticComponents};
use crate::entities::remove_components::{ComponentSet, RemoveComponents, StaticComponentSet};
use crate::entities::{Component, ComponentId, EditEntity, Entities, EntityPtr, SpawnBatch};
use crate::query::{Query, QueryIter};
use crate::Entity;

/// A collection of entities.
///
/// Unlike the regular [`Entities`] collection, the [`World`] allows safe mutable and shared
/// access to its content through the use of regular Rust references.
///
/// When the world is access mutably, it is guaranteed that no other reference to the world
/// exists, and it is therefor possible to access any of its entities mutably.
pub struct World(Entities);

impl World {
    /// Creates a new emtpy [`World`] instance.
    #[inline]
    pub const fn new() -> Self {
        Self(Entities::new())
    }

    /// Spawns a new entity with the provided components.
    #[inline]
    pub fn spawn<C>(&mut self, components: C) -> EntityMut
    where
        C: Components + Send + Sync,
    {
        let id = self.0.spawn(components);

        EntityMut {
            entity: id,
            entities: &mut self.0,
        }
    }

    /// Spawns a batch of new entities with the provided components.
    #[inline]
    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatch<I::IntoIter>
    where
        I: IntoIterator,
        I::Item: StaticComponents + Send + Sync,
    {
        self.0.spawn_batch(iter)
    }

    /// Returns a reference to one of the entities in this [`World`].
    ///
    /// # Panics
    ///
    /// This function panics if the provided [`Entity`] does not exist in this [`World`].
    #[inline]
    #[track_caller]
    pub fn entity(&self, entity: Entity) -> EntityRef {
        self.try_entity(entity).expect("entity does not exist")
    }

    /// Returns an exclusive reference to one of the entities in this [`World`].
    ///
    /// # Panics
    ///
    /// This function panics if the provided [`Entity`] does not exist in this [`World`].
    #[inline]
    #[track_caller]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut {
        self.try_entity_mut(entity).expect("entity does not exist")
    }

    /// Returns a reference to one of the entities in this [`World`].
    ///
    /// Returns `None` if the provided [`Entity`] does not exist in this [`World`].
    pub fn try_entity(&self, entity: Entity) -> Option<EntityRef> {
        if self.0.is_alive(entity) {
            Some(EntityRef {
                entity,
                entities: &self.0,
            })
        } else {
            None
        }
    }

    /// Returns an exclusive reference to one of the entities in this [`World`].
    ///
    /// Returns `None` if the provided [`Entity`] does not exist in this [`World`].
    pub fn try_entity_mut(&mut self, entity: Entity) -> Option<EntityMut> {
        if self.0.is_alive(entity) {
            Some(EntityMut {
                entity,
                entities: &mut self.0,
            })
        } else {
            None
        }
    }

    /// Returns whether the provided [`Entity`] is alive in this [`World`] or not.
    #[inline(always)]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.0.is_alive(entity)
    }

    /// Queries the [`World`] for entities that match the provided query.
    #[inline]
    pub fn query<'e, Q: Query<'e>>(&'e mut self) -> QueryIter<'e, Q> {
        unsafe { QueryIter::new_unchecked(&self.0) }
    }
}

/// A reference to an entity in a [`World`].
#[derive(Copy, Clone)]
pub struct EntityRef<'a> {
    entity: Entity,
    entities: &'a Entities,
}

impl<'a> EntityRef<'a> {
    /// Returns the ID of the entity.
    #[inline(always)]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Returns the raw [`EntityPtr`] of the entity.
    #[inline]
    pub fn as_ptr(&self) -> EntityPtr<'a> {
        unsafe { self.entities.get(self.entity.index()) }
    }

    /// Returns the number of components in the entity.
    #[inline]
    pub fn component_count(&self) -> usize {
        self.as_ptr().layout().component_count()
    }

    /// Gets a shared reference to a specific component.
    ///
    /// If the entity does not have the component, this function returns `None`.
    #[inline]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Component,
    {
        unsafe { self.as_ptr().get_raw::<T>().as_ref() }
    }

    /// Returns whether the entity has a component of the provided type.
    #[inline]
    pub fn has<T>(&self) -> bool
    where
        T: Component,
    {
        self.as_ptr().has_component(ComponentId::of::<T>())
    }
}

/// An exclusive reference to an entity in a [`World`].
pub struct EntityMut<'a> {
    entity: Entity,
    entities: &'a mut Entities,
}

impl<'a> EntityMut<'a> {
    /// Returns the ID of the entity.
    #[inline(always)]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Returns the raw [`EntityPtr`] of the entity.
    #[inline]
    pub fn as_ptr(&self) -> EntityPtr {
        unsafe { self.entities.get(self.entity.index()) }
    }

    /// Returns the number of components in the entity.
    #[inline]
    pub fn component_count(&self) -> usize {
        self.as_ptr().layout().component_count()
    }

    /// Gets a shared reference to a specific component.
    ///
    /// If the entity does not have the component, this function returns `None`.
    #[inline]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Component,
    {
        unsafe { self.as_ptr().get_raw::<T>().as_ref() }
    }

    /// Gets a mutable reference to a specific component.
    ///
    /// If the entity does not have the component, this function returns `None`.
    #[inline]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Component,
    {
        unsafe { self.as_ptr().get_raw::<T>().as_mut() }
    }

    /// Gets a mutable reference to a specific component.
    ///
    /// This function consumes the [`EntityMut`] instance, returning a mutable reference to the
    /// component with the lifetime of the [`EntityMut`] itself.
    #[inline]
    pub fn into_mut<T>(self) -> Option<&'a mut T>
    where
        T: Component,
    {
        unsafe { self.as_ptr().get_raw::<T>().as_mut() }
    }

    /// Returns whether the entity has a component of the provided type.
    #[inline]
    pub fn has<T>(&self) -> bool
    where
        T: Component,
    {
        self.as_ptr().has_component(ComponentId::of::<T>())
    }

    /// Edits this entity, applying the provided edit function.
    pub fn edit<E>(&mut self, edit: E) -> E::Output
    where
        E: EditEntity,
    {
        unsafe { self.entities.edit(self.entity.index(), edit) }
    }

    /// Adds components to the entity.
    ///
    /// If the entity already has any of the provided components, they will be replaced.
    #[inline]
    pub fn add<T>(&mut self, components: T)
    where
        T: Components,
    {
        self.edit(AddComponents(components))
    }

    /// Removes components from the entity.
    ///
    /// If the entity does not have any of the provided components, this function does nothing.
    #[inline]
    pub fn remove_with_set<S>(&mut self, set: &S)
    where
        S: ComponentSet,
    {
        self.edit(RemoveComponents(set))
    }

    /// Removes components from the entity.
    ///
    /// If the entity does not have any of the provided components, this function does nothing.
    #[inline]
    pub fn remove<C>(&mut self)
    where
        StaticComponentSet<C>: ComponentSet,
    {
        self.remove_with_set(&StaticComponentSet::default())
    }

    /// Despawns the entity.
    #[inline]
    pub fn despawn(self) {
        unsafe { self.entities.despawn(self.entity.index()) };
    }
}
