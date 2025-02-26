use {
    crate::{
        OpaquePtr, Uuid,
        app::{Event, EventContext, EventHandlers, FromApp, Global, Globals, RawEventContext},
        entities::{ComponentList, Entities, EntityId, EntityMut, EntityRef},
        system::{IntoSystem, Schedule, System},
    },
    std::mem::ManuallyDrop,
};

/// A map from [`Uuid`] to a value.
type Schedules = hashbrown::HashMap<Uuid, Schedule, foldhash::fast::FixedState>;

/// Contains the complete state of the application.
///
/// This type allows dispatching events and managing the application's resources.
#[derive(Default)]
pub struct App {
    /// The globals that are shared across the application.
    globals: Globals,
    /// Stores the entities for the application.
    entities: Entities,
    /// The event handlers that are registered with the application.
    event_handlers: EventHandlers,
    /// The schedules that the application can run.
    schedules: Schedules,
}

impl App {
    // ========================================================================================== //
    // GLOBALS                                                                                    //
    // ========================================================================================== //

    /// Returns a shared reference to the [`Globals`] collection.
    #[inline(always)]
    pub fn globals(&self) -> &Globals {
        &self.globals
    }

    /// Returns an exclusive reference to the [`Globals`] collection.
    ///
    /// # Safety
    ///
    /// The caller must not move out of the [`Globals`] instance out of the mutable reference.
    #[inline(always)]
    pub unsafe fn globals_mut(&mut self) -> &mut Globals {
        &mut self.globals
    }

    /// Registers a global resource with the application.
    ///
    /// # Panics
    ///
    /// This function pancis if the resource has already been registered.
    #[track_caller]
    pub fn register_global<G: Global>(&mut self, global: G) {
        self.globals.register(Box::new(global))
    }

    /// Initializes a global resource with the application.
    ///
    /// This function uses the type's [`FromApp`] implementation to create the global resource. If
    /// the global has already been registered, this function will do nothing.
    pub fn init_global<G: Global + FromApp>(&mut self) {
        if self.globals.get_raw_mut(G::UUID).is_none() {
            let b = Box::new(G::from_app(self));
            self.globals.register(b)
        }
    }

    /// Retrieves a global resource from the application.
    ///
    /// # Returns
    ///
    /// If the resource is found, this function returns a reference to it. Otherwise, it returns
    /// `None`.
    #[inline]
    pub fn get_global<G: Global>(&self) -> Option<&G> {
        self.globals.try_get::<G>()
    }

    /// Retrieves a mutable reference to a global resource from the application.
    ///
    /// # Returns
    ///
    /// If the resource is found, this function returns a mutable reference to it. Otherwise, it
    /// returns `None`.
    #[inline]
    pub fn get_global_mut<G: Global>(&mut self) -> Option<&mut G> {
        self.globals.try_get_mut::<G>()
    }

    /// Retrieves a global resource from the application.
    ///
    /// # Panics
    ///
    /// This function panics if the resource is not found.
    #[inline]
    #[track_caller]
    pub fn global<G: Global>(&self) -> &G {
        self.globals.get::<G>()
    }

    /// Retrieves a mutable reference to a global resource from the application.
    ///
    /// # Panics
    ///
    /// This function panics if the resource is not found.
    #[inline]
    #[track_caller]
    pub fn global_mut<G: Global>(&mut self) -> &mut G {
        self.globals.get_mut::<G>()
    }

    // ========================================================================================== //
    // ENTITIES                                                                                   //
    // ========================================================================================== //

    /// Returns a shared reference to the [`Entities`] collection.
    #[inline(always)]
    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    /// Returns an exclusive reference to the [`Entities`] collection.
    ///
    /// # Safety
    ///
    /// The caller must not move out of the [`Entities`] instance out of the mutable reference.
    #[inline(always)]
    pub unsafe fn entities_mut(&mut self) -> &mut Entities {
        &mut self.entities
    }

    /// Retrieves an entity from the application.
    ///
    /// # Returns
    ///
    /// If the entity exists, this function returns a reference to it. Otherwise, it returns `None`.
    #[inline]
    pub fn get_entity(&self, entity: EntityId) -> Option<EntityRef> {
        self.entities.get_entity(entity)
    }

    /// Retrieves an entity from the application.
    ///
    /// # Returns
    ///
    /// If the entity exists, this function returns a mutable reference to it. Otherwise, it returns
    /// `None`.
    #[inline]
    pub fn get_entity_mut(&mut self, entity: EntityId) -> Option<EntityMut> {
        self.entities.get_entity_mut(entity)
    }

    /// Spawns a new entity in the application.
    ///
    /// # Returns
    ///
    /// This function returns an [`EntityMut`] reference that can be used to access the entity's
    /// components.
    pub fn spawn(&mut self, components: impl ComponentList) -> EntityMut {
        self.entities.spawn(components)
    }

    /// Despawns an entity from the application.
    ///
    /// # Panics
    ///
    /// This function panics if the entity does not exist.
    #[track_caller]
    pub fn despawn(&mut self, entity: EntityId) {
        self.entities.entity_mut(entity).despawn();
    }

    /// Returns the entity with the provided ID.
    ///
    /// # Panics
    ///
    /// This function panics if the entity does not exist.
    #[track_caller]
    pub fn entity_mut(&mut self, entity: EntityId) -> EntityMut {
        self.entities.entity_mut(entity)
    }

    /// Returns the entity with the provided ID.
    ///
    /// # Panics
    ///
    /// This function panics if the entity does not exist.
    #[track_caller]
    pub fn entity(&self, entity: EntityId) -> EntityRef {
        self.entities.entity(entity)
    }

    // ========================================================================================== //
    // EVENTS                                                                                     //
    // ========================================================================================== //

    /// Calls the provided closure with the event handlers of the application.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the event handlers are not replaced by ones that are not
    /// associated with this application.
    unsafe fn with_event_handlers<R>(
        &mut self,
        f: impl FnOnce(&mut Self, &mut EventHandlers) -> R,
    ) -> R {
        struct Guard<'a> {
            app: &'a mut App,
            event_handlers: ManuallyDrop<EventHandlers>,
        }

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                let event_handlers = unsafe { ManuallyDrop::take(&mut self.event_handlers) };
                self.app.event_handlers = event_handlers;
            }
        }

        let event_handlers = ManuallyDrop::new(std::mem::take(&mut self.event_handlers));
        let mut guard = Guard {
            app: self,
            event_handlers,
        };

        f(guard.app, &mut guard.event_handlers)
    }

    /// Triggers an event on the application on the provided entity.
    pub fn trigger_event<E: Event>(&mut self, target: EntityId, event: &mut E) {
        let mut propagate = true;

        let context: EventContext<'_, E> = unsafe {
            EventContext::from_raw(RawEventContext {
                current: target,
                target,
                event: OpaquePtr::from_mut(event),
                propagate: &mut propagate,
            })
        };

        unsafe {
            self.with_event_handlers(|app, event_handlers| event_handlers.trigger(context, app))
        }
    }

    /// Adds an event handler to the application.
    pub fn add_event_handler<E, S, M>(&mut self, handler: S)
    where
        E: Event,
        S: IntoSystem<M>,
        S::System: for<'a> System<In<'a> = EventContext<'a, E>, Out = ()>,
    {
        unsafe {
            let handler = handler.into_system(self);
            self.event_handlers.insert_global(handler);
        }
    }

    /// Adds an event handler to the application. The event handler will be scoped to the provided
    /// entity.
    pub fn add_scoped_event_handler<E, S, M>(&mut self, entity: EntityId, handler: S)
    where
        E: Event,
        S: IntoSystem<M>,
        S::System: for<'a> System<In<'a> = EventContext<'a, E>, Out = ()>,
    {
        unsafe {
            let handler = handler.into_system(self);
            self.event_handlers.insert_scoped(entity, handler);
        }
    }

    // ========================================================================================== //
    // SCHEDULE & SYSTEM MANAGEMENT                                                               //
    // ========================================================================================== //

    /// Ensures that a schedule with the provided UUID exists.
    pub fn init_schedule(&mut self, schedule: Uuid) {
        self.schedules.entry(schedule).or_default();
    }

    /// Calls the provided closure with the schedule that has the provided UUID.
    ///
    /// # Panics
    ///
    /// This function panics if the schedule does not exist.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the schedule will not be replaced by one that's not associated
    /// with this application.
    ///
    /// # Returns
    ///
    /// Whatever the closure returns.
    #[track_caller]
    pub unsafe fn with_schedule<R>(
        &mut self,
        schedule: Uuid,
        f: impl FnOnce(&mut Self, &mut Schedule) -> R,
    ) -> R {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn missing_schedule(uuid: Uuid) -> ! {
            panic!("Schedule with UUID {uuid:?} does not exist");
        }

        struct Guard<'a> {
            schedule_uuid: Uuid,
            schedule: ManuallyDrop<Schedule>,
            app: &'a mut App,
        }

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                let schedule = unsafe { ManuallyDrop::take(&mut self.schedule) };

                assert!(
                    self.app
                        .schedules
                        .insert(self.schedule_uuid, schedule)
                        .is_none(),
                    "Schedule with UUID {:?} was replaced while being accessed",
                    self.schedule_uuid
                );
            }
        }

        let schedule_obj = self
            .schedules
            .remove(&schedule)
            .unwrap_or_else(|| missing_schedule(schedule));

        let mut guard = Guard {
            schedule_uuid: schedule,
            schedule: ManuallyDrop::new(schedule_obj),
            app: self,
        };

        f(guard.app, &mut guard.schedule)
    }

    /// Adds a system to the provided schedule.
    #[track_caller]
    pub fn add_system<S, M>(&mut self, schedule: Uuid, system: S)
    where
        S: IntoSystem<M>,
        S::System: for<'a> System<In<'a> = (), Out = ()>,
    {
        unsafe {
            self.with_schedule(schedule, |app, schedule| {
                schedule.add_system(system.into_system(app))
            });
        }
    }

    /// Runs the schedule with the given ID.
    ///
    /// If the schedule does not exist, this function does nothing.
    #[track_caller]
    pub fn run_schedule(&mut self, schedule: Uuid) {
        unsafe {
            self.with_schedule(schedule, |app, schedule| schedule.run(&(), app));
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App {{ .. }}")
    }
}
