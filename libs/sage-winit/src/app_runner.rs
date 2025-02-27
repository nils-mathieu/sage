use {
    crate::{EventLoopGlobal, events, window::Window},
    pollster::FutureExt,
    sage_core::{RENDER_SCHEDULE, TypeUuid, UPDATE_SCHEDULE, app::App, entities::EntityId},
    sage_wgpu::{OutputTarget, PendingCommandBuffers, Renderer, wgpu},
    std::sync::Arc,
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{DeviceEvent, DeviceId, WindowEvent},
        event_loop::ActiveEventLoop,
        window::{WindowAttributes, WindowId},
    },
};

/// Stores the state associated with a window, but is not part of the ECS.
struct WindowState {
    /// The ID of the entity that has the [`Window`] component within the ECS.
    entity: EntityId,

    /// The surface that is used to render to the window.
    ///
    /// This may be `None` when the surface is not available.
    surface: Option<wgpu::Surface<'static>>,

    /// The surface size.
    size: PhysicalSize<u32>,

    /// A reference to the winit window object.
    window: Arc<winit::window::Window>,

    /// Whether the surface needs to be re-configured.
    needs_configuration: bool,
}

/// Wraps an [`App`] provided by the user and runs allows it to run using the [`winit`] event loop.
///
/// This type implements the [`ApplicationHandler`] trait, which allows it to be used as the
/// callback object for the [`winit`] event loop (see [`winit::event_loop::EventLoop::run_app`]).
pub struct AppRunner {
    /// The initialization function.
    ///
    /// When `Some`, the application has been initialized. When `None`, the application has not been
    /// initialized.
    #[allow(clippy::type_complexity)]
    init_fn: Option<(WindowAttributes, Box<dyn FnOnce(&mut App)>)>,

    /// The user-specific application that we are wrapping.
    app: App,

    /// The windows that the application is currently managing.
    ///
    /// This hash map is used to convert [`WindowId`]s from [`winit`] into the [`EntityId`] that
    /// represents the window in the application's entity-component system.
    windows: hashbrown::HashMap<WindowId, WindowState, foldhash::fast::FixedState>,

    /// Whether it is possible to create surfaces for the windows.
    surfaces_available: bool,
}

impl AppRunner {
    /// Creates a new [`AppRunner`] instance from the provided [`App`].
    pub fn new(attrs: WindowAttributes, init_fn: Box<dyn FnOnce(&mut App)>) -> Self {
        AppRunner {
            init_fn: Some((attrs, init_fn)),
            app: App::default(),
            windows: hashbrown::HashMap::default(),
            surfaces_available: false,
        }
    }

    /// Reads the state that the user might have modified during the last time they had
    /// control.
    fn end_of_user_flow(&mut self, event_loop: &ActiveEventLoop) {
        self.app.flush();

        // Close the window whose entity/component has been removed.
        self.windows.retain(|&window_id, state| {
            self.app
                .get_entity(state.entity)
                .and_then(|entity| entity.try_get::<Window>())
                .is_some_and(|window| window.winit_window().id() == window_id)
        });

        let global = self.app.global_mut::<EventLoopGlobal>();

        if global.exit_requested() {
            event_loop.exit();
        }
        if let Some(windows) = global.take_pending_windows() {
            for (entity, attrs) in windows {
                let winit_window: Arc<winit::window::Window> = event_loop
                    .create_window(attrs)
                    .expect("Failed to create `winit` window")
                    .into();

                self.windows.insert(
                    winit_window.id(),
                    WindowState {
                        entity,
                        surface: self
                            .surfaces_available
                            .then(|| create_surface(self.app.global(), winit_window.clone())),
                        size: winit_window.inner_size(),
                        window: winit_window.clone(),
                        needs_configuration: true,
                    },
                );

                self.app
                    .entity_mut(entity)
                    .insert(Window::new(winit_window));
            }
        }

        // If no more windows are open, exit the event loop.
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}

impl<T: 'static> ApplicationHandler<T> for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.surfaces_available = true;

        if let Some((attrs, init_fn)) = self.init_fn.take() {
            // Create the main window.
            let main_window: Arc<winit::window::Window> = event_loop
                .create_window(attrs)
                .expect("Failed to create `winit` window")
                .into();

            let (renderer, surface) = Renderer::from_surface_target(main_window.clone()).block_on();

            self.windows.insert(
                main_window.id(),
                WindowState {
                    entity: self.app.spawn(Window::new(main_window.clone())).id(),
                    surface: Some(surface),
                    size: main_window.inner_size(),
                    window: main_window.clone(),
                    needs_configuration: true,
                },
            );

            // Initializes the global resources.
            self.app.register_global(renderer);
            self.app.init_global::<EventLoopGlobal>();
            self.app.init_global::<OutputTarget>();
            self.app.init_global::<PendingCommandBuffers>();

            // Run the startup schedule.
            init_fn(&mut self.app);
            self.end_of_user_flow(event_loop);
        } else {
            // Re-create lost surfaces.
            let renderer = self.app.global::<Renderer>();
            for state in self.windows.values_mut() {
                state.surface = Some(create_surface(renderer, state.window.clone()));
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.surfaces_available = false;
        for state in self.windows.values_mut() {
            state.surface = None;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.windows.get_mut(&window_id) else {
            // For some reason, we received an event for a window that we don't know about.
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                let mut event = events::CloseRequested::default();
                self.app.trigger_event(state.entity, &mut event);
                if !event.is_prevented() {
                    self.app.despawn(state.entity);
                }
            }
            WindowEvent::Resized(new_size) => {
                state.size = new_size;
                self.app
                    .entity_mut(state.entity)
                    .get_mut::<Window>()
                    .surface_size = new_size;
                self.app
                    .trigger_event(state.entity, &mut events::SurfaceResized(new_size));
                state.needs_configuration = true;
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
            } => {
                self.app
                    .entity_mut(state.entity)
                    .get_mut::<Window>()
                    .scale_factor = scale_factor;
                self.app.trigger_event(
                    state.entity,
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
                    .entity_mut(state.entity)
                    .get_mut::<Window>()
                    .pointer_position = Some(position);
                self.app.trigger_event(
                    state.entity,
                    &mut events::PointerMoved {
                        position,
                        device_id,
                    },
                );
            }
            WindowEvent::CursorEntered { device_id } => {
                self.app
                    .trigger_event(state.entity, &mut events::PointerEntered { device_id });
            }
            WindowEvent::CursorLeft { device_id } => {
                self.app
                    .entity_mut(state.entity)
                    .get_mut::<Window>()
                    .pointer_position = None;
                self.app
                    .trigger_event(state.entity, &mut events::PointerLeft { device_id });
            }
            WindowEvent::Focused(now_focused) => {
                self.app
                    .entity_mut(state.entity)
                    .get_mut::<Window>()
                    .focused = now_focused;
                self.app
                    .trigger_event(state.entity, &mut events::Focused(now_focused));
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                self.app.trigger_event(
                    state.entity,
                    &mut events::KeyboardInput {
                        device_id,
                        inner: event,
                        is_synthetic,
                    },
                );
            }
            WindowEvent::RedrawRequested => {
                self.app.run_schedule(UPDATE_SCHEDULE);

                if let Some(surface) = state.surface.as_ref() {
                    if state.needs_configuration {
                        state.needs_configuration = false;

                        let renderer = self.app.global::<Renderer>();
                        surface.configure(
                            renderer.device(),
                            &wgpu::SurfaceConfiguration {
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                format: renderer.output_format(),
                                width: state.size.width,
                                height: state.size.height,
                                present_mode: wgpu::PresentMode::AutoVsync,
                                desired_maximum_frame_latency: 1,
                                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                                view_formats: vec![],
                            },
                        );
                    }

                    let frame = surface
                        .get_current_texture()
                        .expect("Failed to acquire swap-chain texture");

                    self.app
                        .global_mut::<OutputTarget>()
                        .populate(frame.texture.create_view(&Default::default()));
                    self.app.run_schedule(RENDER_SCHEDULE);
                    self.app.global_mut::<OutputTarget>().clear();

                    // SAFETY:
                    //  `PendingCommandBuffers` is not the same resource as `Renderer`, ensuring
                    //  that we don't alias references.
                    let cbs = unsafe {
                        self.app
                            .globals()
                            .get_raw(PendingCommandBuffers::UUID)
                            .expect("Resource `PendingCommandBuffers` is missing")
                            .data()
                            .as_mut::<PendingCommandBuffers>()
                            .drain()
                    };
                    let renderer = unsafe {
                        self.app
                            .globals()
                            .get_raw(Renderer::UUID)
                            .expect("Resource `Renderer` is missing")
                            .data()
                            .as_ref::<Renderer>()
                    };

                    renderer.queue().submit(cbs);

                    state.window.pre_present_notify();
                    frame.present();
                }
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

/// Creates a new surface from the provided window.
fn create_surface(
    renderer: &Renderer,
    window: Arc<winit::window::Window>,
) -> wgpu::Surface<'static> {
    let s = renderer
        .instance()
        .create_surface(window)
        .expect("Failed to create surface");

    assert!(
        s.get_capabilities(renderer.adapter())
            .formats
            .contains(&renderer.output_format()),
        "The created surface does not support the output format of the renderer",
    );

    s
}
