use sage_core::{TypeUuid, Uuid, app::Global};

/// A **global** containing the basic rendering context for the whole application.
///
/// This includes stuff like the [`wgpu::Instance`] or the [`wgpu::Device`].
pub struct Renderer {
    /// The instance that was used to create the device.
    ///
    /// This is kept around in order to create new surfaces when needed.
    instance: wgpu::Instance,

    /// The GPU adapter that was selected to create the device.
    adapter: wgpu::Adapter,

    /// The default output format of the whole rendering pipeline.
    ///
    /// All surfaces created must support this format.
    output_format: wgpu::TextureFormat,

    /// The device that was created for rendering.
    ///
    /// This is associated with the adapter and is used to create all other resources.
    device: wgpu::Device,

    /// The device queue that is used to submit commands to the GPU.
    ///
    /// This is associated with the device.
    queue: wgpu::Queue,

    /// If available, the pipeline cache which can be used to speed up pipeline creation.
    pipeline_cache: Option<wgpu::PipelineCache>,
}

impl Renderer {
    /// Creates a new [`Renderer`] from the provided [`wgpu::Instance`] and window.
    ///
    /// # Returns
    ///
    /// This function returns both a [`Renderer`] and a [`wgpu::Surface`] for the provided window.
    pub async fn from_surface_target<'a>(
        window: impl Into<wgpu::SurfaceTarget<'a>>,
    ) -> (Self, wgpu::Surface<'a>) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self::from_instance(instance, window.into()).await
    }

    /// Creates a new [`Renderer`] from the provided [`wgpu::Instance`] and [`wgpu::Surface`].
    ///
    /// # Returns
    ///
    /// This function returns both a [`Renderer`] and a [`wgpu::Surface`] for the provided window.
    pub async fn from_instance(
        instance: wgpu::Instance,
        window: wgpu::SurfaceTarget<'_>,
    ) -> (Self, wgpu::Surface) {
        let surface = instance
            .create_surface(window)
            .unwrap_or_else(|_| panic!("Failed to create surface"));
        let renderer = Self::from_surface(instance, &surface).await;
        (renderer, surface)
    }

    /// Creates a new [`Renderer`] from the provided [`wgpu::Instance`] and [`wgpu::Surface`].
    pub async fn from_surface(instance: wgpu::Instance, surface: &wgpu::Surface<'_>) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(surface),
            })
            .await
            .unwrap_or_else(|| panic!("Found no suitable GPU adapter"));

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_limits: wgpu::Limits::default(),
                    required_features: wgpu::Features::empty(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to establish connection with GPU device"));

        let surface_caps = surface.get_capabilities(&adapter);

        let output_format = *surface_caps
            .formats
            .iter()
            .find(|x| x.is_srgb())
            .or(surface_caps.formats.first())
            .unwrap_or_else(|| {
                panic!("The surface is not compatible with the selected GPU adapter");
            });

        Self {
            instance,
            adapter,
            output_format,
            device,
            queue,
            pipeline_cache: None,
        }
    }

    /// Returns the [`wgpu::Instance`] representing the rendering context.
    #[inline(always)]
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Returns the [`wgpu::Adapter`] that was selected for rendering.
    #[inline(always)]
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// Returns the output format of the whole rendering pipeline.
    #[inline(always)]
    pub fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }

    /// Returns the [`wgpu::Device`] that was created for rendering.
    #[inline(always)]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the [`wgpu::Queue`] that is used to submit commands to the GPU.
    #[inline(always)]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// If available, returns the pipeline cache.
    ///
    /// It can be used to speed up pipeline creation.
    #[inline]
    pub fn pipeline_cache(&self) -> Option<&wgpu::PipelineCache> {
        self.pipeline_cache.as_ref()
    }
}

unsafe impl TypeUuid for Renderer {
    const UUID: Uuid = Uuid::from_u128(0x6ee30dc962cde74dcb54c76e402a560d);
}

impl Global for Renderer {}
