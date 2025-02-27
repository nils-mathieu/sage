//! UI rendering module.

mod rect;
pub use self::rect::*;

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
    std::{num::NonZero, ops::Range},
};

/// An error that might occur when rasterizing a glyph.
#[derive(Debug, Clone, Copy)]
pub enum GlyphError {
    /// The requested font is missing from the font context.
    MissingFont,
    /// The requested glyph is missing from the font.
    MissingGlyph,
    /// The atlas responsible for holding the glyphs is full.
    AtlasFull,
}

/// The kind of an UI command.
#[derive(Debug, Clone, PartialEq, Eq)]
enum UiCommandKind {
    /// A collection of glyphs.
    Glyphs,
    /// A collection of rectangles.
    Rects,
}

/// A potentially batched rendering command.
#[derive(Debug, Clone)]
struct UiCommand {
    /// The Z-index of the command.
    pub z_index: i32,
    /// The range of items to batch.
    pub range: Range<u32>,
    /// The kind of the command.
    pub kind: UiCommandKind,
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
    rects: Vec<RectInstance>,
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
    /// The commands that need to be executed.
    ui_commands: Vec<UiCommand>,
}

impl UiPass {
    /// Adds a rectangle to the list of rectangles.
    ///
    /// # Remarks
    ///
    /// This function won't add a rendering command to actually draw the rectangle. The caller must
    /// call [`submit_rects`](UiPass::submit_rects) to actually draw the rectangles.
    #[inline]
    pub fn add_rect_no_draw(&mut self, rect: RectInstance) {
        self.rects.push(rect);
    }

    /// Adds a rectangle to the list of rectangles to be rendered.
    ///
    /// # Remarks
    ///
    /// This function won't add a rendering command to actually draw the rectangle. The caller must
    /// call [`submit_rects`](UiPass::submit_rects) to actually draw the rectangles.
    #[inline]
    pub fn add_rects_no_draw(&mut self, rects: &[RectInstance]) {
        self.rects.extend_from_slice(rects);
    }

    /// Attempts to batch the last command for which `is_same_kind` returns `true` with the
    /// provided new end index.
    fn submit_batch(&mut self, end_index: u32, z_index: i32, kind: UiCommandKind) {
        if let Some(cmd) = self.ui_commands.iter_mut().find(|x| kind == x.kind) {
            if cmd.z_index == z_index {
                // We can batch the rectangles. They are on the same z-index.
                cmd.range.end = end_index;
            } else {
                // We can't batch the rectangles. They are on different z-indices.
                let start = cmd.range.start;
                let end = end_index;

                if start == end {
                    return;
                }

                self.ui_commands.push(UiCommand {
                    z_index,
                    range: start..end,
                    kind,
                });
            }
        } else {
            // This is the first command.
            let start = 0;
            let end = end_index;

            if start == end {
                return;
            }

            self.ui_commands.push(UiCommand {
                z_index,
                range: start..end,
                kind,
            });
        }
    }

    /// Submits the rectangles to be rendered.
    ///
    /// This shoulld be called after rectangles like [`add_rect_no_draw`](UiPass::add_rect_no_draw)
    /// or [`add_rects_no_draw`](UiPass::add_rects_no_draw) have been called.
    pub fn submit_rects(&mut self, z_index: i32) {
        self.submit_batch(self.rects.len() as u32, z_index, UiCommandKind::Rects);
    }

    /// Appends a single glyph to the list of glyphs to be rendered.
    ///
    /// # Remarks
    ///
    /// This function does not add a rendering command to actually draw the glyphs. The caller must
    /// call [`submit_glyphs`](UiPass::submit_glyphs) to actually draw the glyphs.
    #[inline]
    pub fn add_glyph_instance_no_draw(&mut self, glyph: GlyphInstance) {
        self.glyphs.push(glyph);
    }

    /// Appends a collection of glyphs to the list of glyphs to be rendered.
    ///
    /// # Remarks
    ///
    /// This function does not add a rendering command to actually draw the glyphs. The caller must
    /// call [`submit_glyphs`](UiPass::submit_glyphs) to actually draw the glyphs.
    #[inline]
    pub fn add_glyphs_instance_no_draw(&mut self, glyphs: &[GlyphInstance]) {
        self.glyphs.extend_from_slice(glyphs);
    }

    /// Adds a rendering command for the last batch of glyphs.
    pub fn submit_glyphs(&mut self, z_index: i32) {
        self.submit_batch(self.glyphs.len() as u32, z_index, UiCommandKind::Glyphs);
    }

    /// Appends a laid-out glyph to the list of glyphs to be rendered.
    ///
    /// # Remarks
    ///
    /// This function will rasterize the glyph and add it to the internal glyph cache if it is not
    /// already present. If the glyph is already present in the cache, it will be reused.
    ///
    /// However, the function won't add a rendering command to actually draw the glyph. The caller
    /// must call [`submit_glyphs`](UiPass::submit_glyphs) to actually draw the glyphs.
    #[allow(clippy::too_many_arguments)]
    pub fn add_glyph_no_draw(
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

        self.add_glyph_instance_no_draw(GlyphInstance {
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
    ///
    /// This function does not add a rendering command to actually draw the glyphs. The caller must
    /// call [`submit_glyphs`](UiPass::submit_glyphs) to actually draw the glyphs.
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_buffer_no_draw(
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
                _ = self.add_glyph_no_draw(
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
            .create_shader_module(wgpu::include_wgsl!("rect.wgsl"));

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
                        buffers: &[RectInstance::LAYOUT],
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
            ui_commands: Vec::new(),
        }
    }
}

/// Prepares the UI pass for rendering.
///
/// This should be called at the begining of the rendering logic.
pub(crate) fn prepare_frame(mut pass: Glob<&mut UiPass>) {
    pass.ui_commands.clear();
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
    let pass = &mut *pass;

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

    pass.ui_commands.sort_unstable_by_key(|cmd| cmd.z_index);

    for cmd in &pass.ui_commands {
        match cmd.kind {
            UiCommandKind::Rects => {
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

                rp.set_pipeline(&pass.rects_pipeline);
                rp.set_vertex_buffer(0, pass.rects_buf.as_ref().unwrap().slice(..));
                rp.draw(0..4, 0..pass.rects.len() as u32);
            }
            UiCommandKind::Glyphs => {
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

                rp.set_pipeline(&pass.glyphs_pipeline);
                rp.set_bind_group(1, pass.text_atlas.bind_group(), &[]);
                rp.set_vertex_buffer(0, pass.glyphs_buf.as_ref().unwrap().slice(..));
                rp.draw(0..4, 0..pass.glyphs.len() as u32);
            }
        }
    }

    drop(rp);

    cbs.append(cb.finish());
}
