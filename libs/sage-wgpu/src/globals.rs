use sage_core::{TypeUuid, Uuid, app::Global};

/// A view into the output of the rendering pipeline.
///
/// This is usually a surface texture, but it might be a concrete image when rendering a screenshot
/// for example.
///
/// The concrete format of the texture can be accessed by reading [`Renderer::output_format`].
///
/// [`Renderer::output_format`]: crate::Renderer::output_format
#[derive(Debug, Default)]
pub struct OutputTarget(Option<wgpu::TextureView>);

impl OutputTarget {
    /// Returns the texture view.
    ///
    /// # Panics
    ///
    /// This function panics if no texture view is available. This often happens because it was
    /// called out of the rendering pipeline.
    #[inline]
    #[track_caller]
    pub fn as_view(&self) -> &wgpu::TextureView {
        self.0
            .as_ref()
            .expect("OutputTarget texture view is not populated")
    }

    /// Populates the touch target with a new texture view.
    #[inline]
    pub fn populate(&mut self, view: wgpu::TextureView) {
        self.0 = Some(view);
    }

    /// Clears the output target.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = None;
    }
}

unsafe impl TypeUuid for OutputTarget {
    const UUID: Uuid = Uuid::from_u128(0x96b6b570b57820dce74d033652b3e7f9);
}

impl Global for OutputTarget {}

/// The list of command buffers that must be executed in order to render the current frame.
#[derive(Debug, Default)]
pub struct PendingCommandBuffers(Vec<wgpu::CommandBuffer>);

impl PendingCommandBuffers {
    /// Appends the provided command buffer to the list.
    pub fn append(&mut self, cb: wgpu::CommandBuffer) {
        self.0.push(cb);
    }

    /// Drains the list of command buffers.
    #[inline]
    pub fn drain(&mut self) -> impl Iterator<Item = wgpu::CommandBuffer> + '_ {
        self.0.drain(..)
    }
}

unsafe impl TypeUuid for PendingCommandBuffers {
    const UUID: Uuid = Uuid::from_u128(0xb826ea9092c2e9f25c29e1c4c63f44e7);
}

impl Global for PendingCommandBuffers {}
