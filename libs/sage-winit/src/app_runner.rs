use {
    crate::{EventLoopGlobal, STARTUP_SCHEDULE, Window, events},
    sage_core::{app::App, entities::EntityId},
    std::sync::Arc,
    winit::{
        application::ApplicationHandler,
        event::{DeviceEvent, DeviceId, WindowEvent},
        event_loop::ActiveEventLoop,
        window::{Window as WinitWindow, WindowId},
    },
};

/// Wraps an [`App`] provided by the user and runs allows it to run using the [`winit`] event loop.
///
/// This type implements the [`ApplicationHandler`] trait, which allows it to be used as the
/// callback object for the [`winit`] event loop (see [`winit::event_loop::EventLoop::run_app`]).
pub struct AppRunner<'a> {
    /// The user-specific application that we are wrapping.
    app: &'a mut App,

    /// Whether the initialization sequence of the application has been completed or not. This is
    /// flipped the first time windows can actually be created.
    initialized: bool,

    /// The windows that the application is currently managing.
    ///
    /// This hash map is used to convert [`WindowId`]s from [`winit`] into the [`EntityId`] that
    /// represents the window in the application's entity-component system.
    windows: hashbrown::HashMap<WindowId, EntityId, foldhash::fast::FixedState>,
}

impl AppRunner<'_> {
    /// Creates a new [`AppRunner`] instance from the provided [`App`].
    pub fn new(app: &mut App) -> AppRunner {
        AppRunner {
            app,
            initialized: false,
            windows: hashbrown::HashMap::default(),
        }
    }

    /// Reads the state that the user might have modified during the last time they had
    /// control.
    fn end_of_user_flow(&mut self, event_loop: &ActiveEventLoop) {
        // Close the window whose entity/component has been removed.
        self.windows.retain(|&window_id, &mut entity_id| {
            self.app
                .get_entity(entity_id)
                .and_then(|entity| entity.try_get::<Window>())
                .is_some_and(|window| window.winit_window().id() == window_id)
        });

        // If no more windows are open, exit the event loop.
        if self.windows.is_empty() {
            event_loop.exit();
        }

        let global = self.app.global_mut::<EventLoopGlobal>();

        if global.exit_requested() {
            event_loop.exit();
        }
        if let Some(windows) = global.take_pending_windows() {
            for (entity, attrs) in windows {
                let winit_window: Arc<WinitWindow> = event_loop
                    .create_window(attrs)
                    .expect("Failed to create `winit` window")
                    .into();
                self.windows.insert(winit_window.id(), entity);
                self.app
                    .entity_mut(entity)
                    .insert(Window::new(winit_window));
            }
        }
    }
}

impl<T: 'static> ApplicationHandler<T> for AppRunner<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.initialized {
            self.app.init_global::<EventLoopGlobal>();
            self.app.run_schedule(STARTUP_SCHEDULE);
            self.end_of_user_flow(event_loop);
            self.initialized = true;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(&entity) = self.windows.get(&window_id) else {
            // For some reason, we received an event for a window that we don't know about.
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                let mut event = events::CloseRequested::default();
                self.app.trigger_event(entity, &mut event);
                if !event.is_prevented() {
                    self.app.despawn(entity);
                }
            }
            WindowEvent::Resized(new_size) => {
                self.app.entity_mut(entity).get_mut::<Window>().surface_size = new_size;
                self.app
                    .trigger_event(entity, &mut events::SurfaceResized(new_size));
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
            } => {
                self.app.entity_mut(entity).get_mut::<Window>().scale_factor = scale_factor;
                self.app.trigger_event(
                    entity,
                    &mut events::ScaleFactorChanged {
                        scale_factor,
                        inner_size_writer,
                    },
                );
            }
            WindowEvent::CursorMoved {
                position,
                device_id,
            } => {
                self.app
                    .entity_mut(entity)
                    .get_mut::<Window>()
                    .pointer_position = Some(position);
                self.app.trigger_event(
                    entity,
                    &mut events::PointerMoved {
                        position,
                        device_id,
                    },
                );
            }
            WindowEvent::CursorEntered { device_id } => {
                self.app
                    .trigger_event(entity, &mut events::PointerEntered { device_id });
            }
            WindowEvent::CursorLeft { device_id } => {
                self.app
                    .entity_mut(entity)
                    .get_mut::<Window>()
                    .pointer_position = None;
                self.app
                    .trigger_event(entity, &mut events::PointerLeft { device_id });
            }
            WindowEvent::Focused(now_focused) => {
                self.app.entity_mut(entity).get_mut::<Window>().focused = now_focused;
                self.app
                    .trigger_event(entity, &mut events::Focused(now_focused));
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                self.app.trigger_event(
                    entity,
                    &mut events::KeyboardInput {
                        device_id,
                        inner: event,
                        is_synthetic,
                    },
                );
            }
            _ => {}
        }

        self.end_of_user_flow(event_loop);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: DeviceEvent,
    ) {
        self.end_of_user_flow(event_loop);
    }
}
