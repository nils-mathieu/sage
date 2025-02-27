use {
    crate::{
        OpaquePtr, TypeUuid, Uuid,
        app::{App, AppCell},
        entities::EntityId,
        schedule::{Schedule, SystemConfig},
        system::{RawSystem, System, SystemAccess, SystemInput},
    },
    std::{
        marker::PhantomData,
        ops::{Deref, DerefMut},
    },
};

/// An event that can be sent to an [`App`].
///
/// [`App`]: crate::app::App
pub trait Event: 'static + Send + Sync + TypeUuid {
    /// Describes how an event propgates to other entities.
    ///
    /// If the event does not actually traverse any part of the application, this type can be set
    /// to `()`.
    type Propagation: EventPropagation;
}

/// Describes how an event propagates from one entity to another.
pub trait EventPropagation {
    /// The view that the event receives when it propagates.
    type View<'w>;

    /// Using the requested view into the current entity, returns the next entity that the event
    /// should traverse to.
    fn propagate(view: Self::View<'_>) -> Option<EntityId>;
}

impl EventPropagation for () {
    type View<'w> = ();

    #[inline]
    fn propagate(_: Self::View<'_>) -> Option<EntityId> {
        None
    }
}

/// An untyped event context that is passed to event handlers.
///
/// This is FFI-safe.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawEventContext {
    /// The original target entity of the event.
    pub target: EntityId,

    /// The current entity that the event is being processed on.
    ///
    /// If the event has traversed to a different entity, this will be different from `target`.
    pub current: EntityId,

    /// Whether the event should continue to propagate to other entities or not.
    pub propagate: *mut bool,

    /// The event that is being processed.
    ///
    /// This is an exclusive reference to the event.
    pub event: OpaquePtr,
}

unsafe impl Send for RawEventContext {}
unsafe impl Sync for RawEventContext {}

impl SystemInput for RawEventContext {
    type Item<'a> = RawEventContext;
}

/// An event context that is passed to event handlers.
#[repr(transparent)]
pub struct EventContext<'a, E> {
    raw: RawEventContext,
    _marker: PhantomData<&'a mut E>,
}

impl<E> EventContext<'_, E> {
    /// Creates a new [`EventContext`] from a raw event context.
    ///
    /// # Safety
    ///
    /// The caller must make sure that the provided raw event context is properly constructed and
    /// references an event of type `E`.
    #[inline]
    pub unsafe fn from_raw(raw: RawEventContext) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Returns the entity that is currently being processed.
    #[inline(always)]
    pub fn current_entity(&self) -> EntityId {
        self.raw.current
    }

    /// Returns the target entity of the event.
    #[inline(always)]
    pub fn target_entity(&self) -> EntityId {
        self.raw.target
    }

    /// Stops the event from propagating to other entities.
    ///
    /// If the event has no propagation path, this function has no effect.
    #[inline(always)]
    pub fn stop_propagation(&mut self) {
        unsafe { *self.raw.propagate = false };
    }

    /// Creates a new [`EventContext`] with a shorter lifetime.
    #[inline]
    pub fn reborrow(&mut self) -> EventContext<'_, E> {
        EventContext {
            raw: self.raw,
            _marker: PhantomData,
        }
    }
}

impl<E: Event> SystemInput for EventContext<'_, E> {
    type Item<'a> = EventContext<'a, E>;
}

impl<E: Event> Deref for EventContext<'_, E> {
    type Target = E;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.raw.event.as_ref() }
    }
}

impl<E: Event> DerefMut for EventContext<'_, E> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.raw.event.as_mut() }
    }
}

/// A collection of event handlers that can be registered with an application.
#[derive(Default)]
pub struct EventHandlers {
    /// Event handlers that are scoped to a specific entity.
    scoped:
        hashbrown::HashMap<(EntityId, Uuid), Schedule<RawEventContext>, foldhash::fast::FixedState>,

    /// Event handlers that are global and can be triggered by any entity.
    global: hashbrown::HashMap<crate::Uuid, Schedule<RawEventContext>, foldhash::fast::FixedState>,
}

impl EventHandlers {
    /// Inserts an event handler into the collection.
    ///
    /// # Parameters
    ///
    /// - `entity`: The entity that the event handler is reading from.
    ///
    /// - `event`: The event that the handler is listening for.
    ///
    /// - `handler`: The handler function itself.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided system is associated with the same application
    /// as all the other systems.
    pub unsafe fn insert_scoped_raw(
        &mut self,
        entity: EntityId,
        event: Uuid,
        handler: RawSystem<RawEventContext>,
    ) {
        let schedule = self.scoped.entry((entity, event)).or_default();
        unsafe { schedule.add_system_raw(SystemConfig::default(), handler) }
    }

    /// Inserts an event handler into the collection.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided system is associated with the same application
    /// as all the other systems.
    pub unsafe fn insert_scoped<S, E: Event>(&mut self, entity: EntityId, handler: S)
    where
        S: System<In = EventContext<'static, E>, Out = ()>,
    {
        unsafe { self.insert_scoped_raw(entity, E::UUID, convert_handler(handler)) }
    }

    /// Inserts an event handler into the collection. The handler will be triggered for all
    /// entities.
    ///
    /// # Parameters
    ///
    /// - `event`: The event that the handler is listening for.
    ///
    /// - `handler`: The handler function itself.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided system is associated with the same application
    /// as all the other systems.
    pub unsafe fn insert_global_raw(&mut self, event: Uuid, handler: RawSystem<RawEventContext>) {
        unsafe {
            self.global
                .entry(event)
                .or_default()
                .add_system_raw(SystemConfig::default(), handler)
        }
    }

    /// Inserts an event handler into the collection. The handler will be triggered for all
    /// entities.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided system is associated with the same application
    /// as all the other systems.
    pub unsafe fn insert_global<S, E: Event>(&mut self, handler: S)
    where
        S: for<'a> System<In = EventContext<'static, E>, Out = ()>,
    {
        unsafe { self.insert_global_raw(E::UUID, convert_handler(handler)) }
    }

    /// Triggers all event handlers for the specified entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided `uuid` corresponds to the event that is referenced
    /// in the [`RawEventContext`].
    ///
    /// Generally, the [`RawEventContext`] must be properly constructed.
    ///
    /// Additionally, the provided application must be the same application that the event handlers
    /// are associated with.
    pub unsafe fn trigger_raw(&mut self, uuid: Uuid, context: RawEventContext, app: &mut App) {
        if let Some(schedule) = self.scoped.get_mut(&(context.current, uuid)) {
            unsafe { schedule.run(&context, app) };
        }

        if let Some(schedule) = self.global.get_mut(&uuid) {
            unsafe { schedule.run(&context, app) };
        }
    }

    /// Triggers all event handlers for the specified event.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided [`App`] is the same application that the event
    /// handlers are associated with.
    pub unsafe fn trigger<E: Event>(&mut self, context: EventContext<E>, app: &mut App) {
        unsafe { self.trigger_raw(E::UUID, context.raw, app) }
    }
}

/// Converts the provided function into a raw event handler function.
fn convert_handler<S, E>(handler: S) -> RawSystem<RawEventContext>
where
    E: Event,
    S: System<In = EventContext<'static, E>, Out = ()>,
{
    struct Wrapper<S>(S);

    unsafe impl<E, S> System for Wrapper<S>
    where
        E: Event,
        S: for<'a> System<In = EventContext<'static, E>, Out = ()>,
    {
        type In = RawEventContext;
        type Out = ();

        #[inline(always)]
        fn access(&self) -> &SystemAccess {
            self.0.access()
        }

        #[inline(always)]
        unsafe fn run(&mut self, input: RawEventContext, app: AppCell) {
            unsafe { self.0.run(EventContext::from_raw(input), app) }
        }

        #[inline(always)]
        unsafe fn apply_deferred(&mut self, app: &mut App) {
            unsafe { self.0.apply_deferred(app) };
        }
    }

    RawSystem::new(Wrapper(handler))
}
