//! UI rendering module.

mod ui_rect;
pub use self::ui_rect::*;

use {
    sage_core::{
        TypeUuid, Uuid,
        app::{App, FromApp, Global},
        system::Glob,
    },
    sage_wgpu::{
        OutputTarget, PendingCommandBuffers, Renderer,
        wgpu::{self, util::DeviceExt},
    },
    std::num::NonZero,
};

pub struct UiPassMarker;

/// The pass that is responsible for rendering UI elements.
pub struct UiPass {
    /// The rectangles that need to be rendered.
    rects: Vec<UiRectInstance>,
    /// The buffer that contains the `UiRectInstance`s to be used on the GPU.
    ///
    /// This is only initialized when there are rectangles to draw.
    rects_buf: Option<wgpu::Buffer>,
    /// The pipeline responsible for drawing `UiRectInstance`s.
    rects_pipeline: wgpu::RenderPipeline,
}

impl UiPass {
    /// Appends a rectangle to the list of rectangles to be rendered.
    pub fn append_rect(&mut self, rect: UiRectInstance) {
        self.rects.push(rect);
    }
}

unsafe impl TypeUuid for UiPass {
    const UUID: Uuid = Uuid::from_u128(0x972260cdc1e50df2bfd128edfb38dc5e);
}

impl Global for UiPass {}

impl FromApp for UiPass {
    fn from_app(app: &mut App) -> Self {
        let renderer = app.global::<Renderer>();

        let ui_pipeline_layout =
            renderer
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("UI PipelineLayout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let rects_shader_module = renderer
            .device()
            .create_shader_module(wgpu::include_wgsl!("ui_rect.wgsl"));

        let rects_pipeline =
            renderer
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("UiRect RenderPipeline"),
                    layout: Some(&ui_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &rects_shader_module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &rects_shader_module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.output_format(),
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: renderer.pipeline_cache(),
                });

        Self {
            rects: Vec::new(),
            rects_buf: None,
            rects_pipeline,
        }
    }
}

/// Prepares the UI pass for rendering.
///
/// This should be called at the begining of the rendering logic.
pub(crate) fn prepare_frame(mut pass: Glob<&mut UiPass>) {
    pass.rects.clear();
}

/// Submits the frame to the GPU.
///
/// This should be called at the end of the rendering logic.
pub(crate) fn submit_frame(
    mut pass: Glob<&mut UiPass>,
    renderer: Glob<&Renderer>,
    target: Glob<&OutputTarget>,
    mut cbs: Glob<&mut PendingCommandBuffers>,
) {
    let mut cb = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    let mut rp = cb.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("UI RenderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.as_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    if !pass.rects.is_empty() {
        pass.rects
            .sort_unstable_by_key(|instance| std::cmp::Reverse(instance.z_index));

        let rects_bytes: &[u8] = bytemuck::cast_slice(&pass.rects);

        if pass
            .rects_buf
            .as_ref()
            .is_none_or(|buf| buf.size() < rects_bytes.len() as u64)
        {
            pass.rects_buf = Some(renderer.device().create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("UiRectInstance Instance Buffer"),
                    contents: rects_bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        } else {
            let buf = pass.rects_buf.as_ref().unwrap();
            let mut buf = renderer
                .queue()
                .write_buffer_with(buf, 0, NonZero::new(rects_bytes.len() as u64).unwrap())
                .unwrap();
            buf.copy_from_slice(rects_bytes);
        }

        rp.set_pipeline(&pass.rects_pipeline);
        rp.draw(0..4, 0..1);
    } else {
        pass.rects_buf = None;
    }

    drop(rp);

    cbs.append(cb.finish());
}
