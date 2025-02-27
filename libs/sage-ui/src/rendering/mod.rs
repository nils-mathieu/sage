//! UI rendering module.

mod ui_rect;
pub use self::ui_rect::*;

mod view;
pub use self::view::*;

pub mod text;

use {
    self::text::{GlyphInstance, GlyphInstanceFlags, TextAtlas},
    glam::{IVec2, UVec2, Vec2},
    sage_color::Srgba8,
    sage_core::{
        TypeUuid, Uuid,
        app::{App, EventContext, FromApp, Global},
        system::Glob,
    },
    sage_wgpu::{
        OutputTarget, PendingCommandBuffers, Renderer,
        wgpu::{self, util::DeviceExt},
    },
    sage_winit::{Window, events::SurfaceResized},
    std::num::NonZero,
};

/// An error that might occur when rasterizing a glyph.
pub enum GlyphError {
    /// The requested font is missing from the font context.
    MissingFont,
    /// The requested glyph is missing from the font.
    MissingGlyph,
    /// The atlas responsible for holding the glyphs is full.
    AtlasFull,
}

/// The pass that is responsible for rendering UI elements.
pub struct UiPass {
    /// The view that is used to render the UI. This is uploaded to the GPU through
    /// `view_buf`.
    view: View,
    /// Indicates that `view` has changed and that it should be re-uploaded to the GPU.
    view_changed: bool,
    /// The buffer responsible for holding the view data.
    view_buf: wgpu::Buffer,
    /// The bind group layout responsible for creating new `view_bind_group`s.
    view_bind_group_layout: wgpu::BindGroupLayout,
    /// The bind group that references the `view_buf`.
    view_bind_group: wgpu::BindGroup,
    /// The rectangles that need to be rendered.
    rects: Vec<UiRectInstance>,
    /// The buffer that contains the `UiRectInstance`s to be used on the GPU.
    ///
    /// This is only initialized when there are rectangles to draw.
    rects_buf: Option<wgpu::Buffer>,
    /// The pipeline responsible for drawing `UiRectInstance`s.
    rects_pipeline: wgpu::RenderPipeline,
    /// The glyphs that need to be rendered on the next frame.
    glyphs: Vec<GlyphInstance>,
    /// The GPU buffer that holds the glyph instances.
    ///
    /// This is `None` if there are no glyphs to render.
    glyphs_buf: Option<wgpu::Buffer>,
    /// The render pipeline that is used to render the glyphs.
    glyphs_pipeline: wgpu::RenderPipeline,
    /// A context needed when rasterizing text.
    swash_scale_context: swash::scale::ScaleContext,
    /// The atlas that contains the rasterized images that will be available to the GPU.
    text_atlas: TextAtlas,
}

impl UiPass {
    /// Appends a rectangle to the list of rectangles to be rendered.
    ///
    /// # Remarks
    ///
    /// This function does not check whether the inserted rectangle is in bounds of the
    /// screen or not. It is up to the caller to cull unnecessary rectangles.
    #[inline]
    pub fn append_rect(&mut self, rect: UiRectInstance) {
        self.rects.push(rect);
    }

    /// Appends a single glyph to the list of glyphs to be rendered.
    ///
    /// # Remarks
    ///
    /// This function does not update the intenral glyph cache, meaning that this will do nothing,
    /// or will render invalid data if the glyph cache is not properly updated to include the
    /// glyph.
    ///
    /// Additionally, this function does not check whether the inserted glyph is in bounds of the
    /// screen or not. It is up to the caller to cull unnecessary glyphs.
    #[inline]
    pub fn append_glyph_instance(&mut self, glyph: GlyphInstance) {
        self.glyphs.push(glyph);
    }

    /// Appends a laid-out glyph to the list of glyphs to be rendered.
    #[allow(clippy::too_many_arguments)]
    pub fn append_glyph(
        &mut self,
        renderer: &Renderer,
        font_system: &mut cosmic_text::FontSystem,
        position: Vec2,
        scale: f32,
        _z_index: i32,
        fallback_color: Srgba8,
        run: &cosmic_text::LayoutRun,
        glyph: &cosmic_text::LayoutGlyph,
    ) -> Result<(), GlyphError> {
        let physical = glyph.physical(position.into(), scale);

        // Rasterize the glyph.

        let cached_glyph = self.text_atlas.get_or_insert(
            renderer.device(),
            renderer.queue(),
            physical.cache_key,
            || {
                let font = font_system
                    .get_font(glyph.font_id)
                    .ok_or(GlyphError::MissingFont)?;

                let info = font_system
                    .db()
                    .face(font.id())
                    .ok_or(GlyphError::MissingFont)?;

                let font_ref = swash::FontRef::from_index(font.data(), info.index as usize)
                    .ok_or(GlyphError::MissingFont)?;

                let mut scaler = self
                    .swash_scale_context
                    .builder(font_ref)
                    .size(glyph.font_size)
                    .build();

                let rasterized_image = swash::scale::Render::new(&[
                    swash::scale::Source::ColorOutline(0),
                    swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
                    swash::scale::Source::Outline,
                    swash::scale::Source::Bitmap(swash::scale::StrikeWith::BestFit),
                ])
                .format(swash::zeno::Format::Alpha)
                .offset(swash::zeno::Vector::new(
                    physical.cache_key.x_bin.as_float(),
                    physical.cache_key.y_bin.as_float(),
                ))
                .transform(
                    if glyph
                        .cache_key_flags
                        .intersects(cosmic_text::CacheKeyFlags::FAKE_ITALIC)
                    {
                        Some(swash::zeno::Transform::skew(
                            swash::zeno::Angle::from_degrees(14.0),
                            swash::zeno::Angle::ZERO,
                        ))
                    } else {
                        None
                    },
                )
                .render(&mut scaler, glyph.glyph_id)
                .ok_or(GlyphError::MissingGlyph)?;

                Ok(rasterized_image)
            },
        )?;

        if cached_glyph.placement.width == 0 || cached_glyph.placement.height == 0 {
            return Ok(());
        }

        let color = match glyph.color_opt {
            Some(color) => Srgba8::rgba(color.r(), color.g(), color.b(), color.a()),
            None => fallback_color,
        };

        let mut flags = GlyphInstanceFlags::empty();
        match cached_glyph.content {
            swash::scale::image::Content::Color => (),
            swash::scale::image::Content::Mask | swash::scale::image::Content::SubpixelMask => {
                flags.insert(GlyphInstanceFlags::MASK_TEXTURE);
            }
        }

        self.append_glyph_instance(GlyphInstance {
            position: IVec2::new(
                physical.x + cached_glyph.placement.left,
                (run.line_height * scale).round() as i32 + physical.y - cached_glyph.placement.top,
            ),
            size: UVec2::new(cached_glyph.placement.width, cached_glyph.placement.height),
            atlas_position: UVec2::new(
                cached_glyph.atlas_rect.min.x as u32,
                cached_glyph.atlas_rect.min.y as u32,
            ),
            color,
            flags,
        });

        Ok(())
    }

    /// Appends the provided text buffer to the list of glyphs to be rendered.
    ///
    /// # Remarks
    ///
    /// This function ignore errors.
    #[allow(clippy::too_many_arguments)]
    pub fn append_text_buffer(
        &mut self,
        renderer: &Renderer,
        font_system: &mut cosmic_text::FontSystem,
        position: Vec2,
        scale: f32,
        z_index: i32,
        fallback_color: Srgba8,
        text: &cosmic_text::Buffer,
    ) {
        for run in text.layout_runs() {
            for glyph in run.glyphs {
                _ = self.append_glyph(
                    renderer,
                    font_system,
                    position,
                    scale,
                    z_index,
                    fallback_color,
                    &run,
                    glyph,
                );
            }
        }
    }
}

unsafe impl TypeUuid for UiPass {
    const UUID: Uuid = Uuid::from_u128(0x972260cdc1e50df2bfd128edfb38dc5e);
}

impl Global for UiPass {}

impl FromApp for UiPass {
    fn from_app(app: &mut App) -> Self {
        let surface_size = app.single_mut::<&Window>().surface_size();
        let renderer = app.global::<Renderer>();

        let view = View {
            resolution: UVec2::new(surface_size.width, surface_size.height),
        };

        let view_buf = renderer.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI View Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            size: View::BUFFER_SIZE.get(),
            mapped_at_creation: false,
        });

        let view_bind_group_layout =
            renderer
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("UI View BindGroupLayout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(View::BUFFER_SIZE),
                        },
                        count: None,
                    }],
                });

        let view_bind_group = renderer
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI View BindGroup"),
                layout: &view_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_buf.as_entire_binding(),
                }],
            });

        let ui_pipeline_layout =
            renderer
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("UI PipelineLayout"),
                    bind_group_layouts: &[&view_bind_group_layout],
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
                        buffers: &[UiRectInstance::LAYOUT],
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

        let text_atlas = TextAtlas::new(renderer.device());

        let glyphs_pipeline_layout =
            renderer
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("UI Glyphs PipelineLayout"),
                    bind_group_layouts: &[&view_bind_group_layout, text_atlas.bind_group_layout()],
                    push_constant_ranges: &[],
                });

        let glyphs_shader_module = renderer
            .device()
            .create_shader_module(wgpu::include_wgsl!("text/glyph.wgsl"));

        let glyphs_pipeline =
            renderer
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("UI Glyphs RenderPipeline"),
                    layout: Some(&glyphs_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &glyphs_shader_module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[GlyphInstance::LAYOUT],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &glyphs_shader_module,
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
            view,
            view_buf,
            view_changed: true,
            view_bind_group_layout,
            view_bind_group,
            rects: Vec::new(),
            rects_buf: None,
            rects_pipeline,
            glyphs: Vec::new(),
            glyphs_buf: None,
            glyphs_pipeline,
            swash_scale_context: swash::scale::ScaleContext::new(),
            text_atlas,
        }
    }
}

/// Prepares the UI pass for rendering.
///
/// This should be called at the begining of the rendering logic.
pub(crate) fn prepare_frame(mut pass: Glob<&mut UiPass>) {
    pass.rects.clear();
    pass.glyphs.clear();
    pass.text_atlas.trim();
}

/// Updates the view resolution when the window is resized.
pub(crate) fn update_view_resolution(
    event: EventContext<SurfaceResized>,
    mut pass: Glob<&mut UiPass>,
) {
    pass.view.resolution = UVec2::new(event.width, event.height);
    pass.view_changed = true;
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
    if pass.view_changed {
        let mut buf = renderer
            .queue()
            .write_buffer_with(&pass.view_buf, 0, View::BUFFER_SIZE)
            .unwrap();
        buf.copy_from_slice(bytemuck::bytes_of(&pass.view));
        drop(buf);

        pass.view_bind_group = renderer
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI View BindGroup"),
                layout: &pass.view_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pass.view_buf.as_entire_binding(),
                }],
            });

        pass.view_changed = false;
    }

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

    rp.set_bind_group(0, &pass.view_bind_group, &[]);

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
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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

        let glyphs_bytes: &[u8] = bytemuck::cast_slice(&pass.glyphs);

        if pass
            .glyphs_buf
            .as_ref()
            .is_none_or(|buf| buf.size() < glyphs_bytes.len() as u64)
        {
            pass.glyphs_buf = Some(renderer.device().create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("GlyphInstance Instance Buffer"),
                    contents: glyphs_bytes,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                },
            ));
        } else {
            let buf = pass.glyphs_buf.as_ref().unwrap();
            let mut buf = renderer
                .queue()
                .write_buffer_with(buf, 0, NonZero::new(glyphs_bytes.len() as u64).unwrap())
                .unwrap();
            buf.copy_from_slice(glyphs_bytes);
        }

        rp.set_pipeline(&pass.rects_pipeline);
        rp.set_vertex_buffer(0, pass.rects_buf.as_ref().unwrap().slice(..));
        rp.draw(0..4, 0..pass.rects.len() as u32);

        rp.set_pipeline(&pass.glyphs_pipeline);
        rp.set_bind_group(1, pass.text_atlas.bind_group(), &[]);
        rp.set_vertex_buffer(0, pass.glyphs_buf.as_ref().unwrap().slice(..));
        rp.draw(0..4, 0..pass.glyphs.len() as u32);
    } else {
        pass.rects_buf = None;
    }

    drop(rp);

    cbs.append(cb.finish());
}
