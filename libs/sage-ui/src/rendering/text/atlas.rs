use {crate::rendering::GlyphError, sage_wgpu::wgpu, swash::scale::image::Image};

/// Stores cached information about a glyph.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// The rectangle in which the glyph is stored.
    pub atlas_rect: etagere::Rectangle,
    /// The placement of the glyph.
    pub placement: swash::zeno::Placement,
    /// The content type of the glyph.
    pub content: swash::scale::image::Content,
}

/// Information about a glyph that is cached in an atlas.
#[derive(Debug, Clone, Copy)]
struct CachedGlyph {
    /// The rectangle in which the glyph is stored.
    pub info: GlyphInfo,
    /// Whether the glyph should be protected from eviction.
    pub used: bool,
    /// The allocation ID of the glyph.
    pub alloc_id: etagere::AllocId,
}

/// An growable atlas that stores images.
struct Atlas {
    /// The texture that the atlas is stored in.
    texture: wgpu::Texture,
    /// A view into `texture`.
    texture_view: wgpu::TextureView,

    /// The allocator used to construct the atlas.
    packer: etagere::BucketedAtlasAllocator,
    /// The cache that maps cache keys to cached glyphs.
    content: lru::LruCache<cosmic_text::CacheKey, CachedGlyph, foldhash::fast::FixedState>,
}

impl Atlas {
    /// Creates a new [`Atlas`] with the given format.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        const INITIAL_SIZE: u32 = 128;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Text TextureAtlas"),
            size: wgpu::Extent3d {
                width: INITIAL_SIZE,
                height: INITIAL_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            texture_view,
            packer: etagere::BucketedAtlasAllocator::new(etagere::Size::new(
                INITIAL_SIZE as i32,
                INITIAL_SIZE as i32,
            )),
            content: lru::LruCache::unbounded_with_hasher(Default::default()),
        }
    }

    /// Allocates a region of the atlas with the given dimensions.
    ///
    /// This function does not attempt to grow the atlas when it is full. Instead, it will
    /// return `None` if the atlas is full.
    pub fn allocate_no_grow(
        &mut self,
        key: cosmic_text::CacheKey,
        placement: swash::zeno::Placement,
        content: swash::scale::image::Content,
    ) -> Option<GlyphInfo> {
        debug_assert!(
            !self.content.contains(&key),
            "Attempted to allocate for a key that was already in the cache",
        );

        loop {
            if let Some(a) = self.packer.allocate(etagere::Size::new(
                placement.width as i32,
                placement.height as i32,
            )) {
                // Success!
                return Some(
                    self.content
                        .get_or_insert_mut(key, || CachedGlyph {
                            info: GlyphInfo {
                                atlas_rect: a.rectangle,
                                placement,
                                content,
                            },
                            used: true,
                            alloc_id: a.id,
                        })
                        .info,
                );
            }

            // Try to evict glyphs until we can allocate the new glyph.

            while let Some((_, entry)) = self.content.peek_lru() {
                if entry.used {
                    // The least recently used glyph is protected. We can't go any further.
                    return None;
                }

                // We can evict this glyph!
                self.packer.deallocate(entry.alloc_id);
                self.content.pop_lru();
            }
        }
    }

    /// Grows the atlas once to accommodate more images.
    ///
    /// # Returns
    ///
    /// This function returns whether the operation was successful.
    ///
    /// In particular, it will return `false` when the atlas cannot grow any further.
    pub fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        let new_size = self
            .texture
            .size()
            .width
            .checked_mul(2)
            .filter(|&x| x < device.limits().max_texture_dimension_2d && x < i32::MAX as u32);
        let Some(new_size) = new_size else {
            return false;
        };

        self.packer
            .grow(etagere::Size::new(new_size as i32, new_size as i32));

        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Text TextureAtlas"),
            size: wgpu::Extent3d {
                width: new_size,
                height: new_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.texture.format(),
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let mut cb = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        cb.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &new_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.texture.size(),
        );

        queue.submit(Some(cb.finish()));

        self.texture = new_texture;

        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        true
    }

    /// Allocates a region of the atlas with the given dimensions.
    ///
    /// # Returns
    ///
    /// Returns the allocated region, as well as whether the atlas was grown to accommodate the
    /// allocation.
    pub fn allocate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: cosmic_text::CacheKey,
        placement: swash::zeno::Placement,
        content: swash::scale::image::Content,
    ) -> Result<(bool, GlyphInfo), GlyphError> {
        if let Some(glyph) = self.allocate_no_grow(key, placement, content) {
            return Ok((false, glyph));
        }

        loop {
            if !self.grow(device, queue) {
                return Err(GlyphError::AtlasFull);
            }

            if let Some(glyph) = self.allocate_no_grow(key, placement, content) {
                return Ok((true, glyph));
            }
        }
    }

    /// Marks all glyphs as unused, making them eligible for eviction.
    pub fn trim(&mut self) {
        for (_, data) in self.content.iter_mut() {
            data.used = false;
        }
    }

    /// Inserts an image in the atlas.
    ///
    /// The caller must ensure that the data layout of the image corresponds to that of the atlas.
    ///
    /// # Returns
    ///
    /// This function returns whether the atlas was resized to accommodate the image.
    pub fn insert_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: cosmic_text::CacheKey,
        image: &Image,
    ) -> Result<(bool, GlyphInfo), GlyphError> {
        let (did_grow, glyph) =
            self.allocate(device, queue, key, image.placement, image.content)?;

        let pixel_size = self
            .texture
            .format()
            .block_copy_size(Some(wgpu::TextureAspect::All))
            .unwrap_or_default();

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: glyph.atlas_rect.min.x as _,
                    y: glyph.atlas_rect.min.y as _,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &image.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.placement.width * pixel_size),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: image.placement.width,
                height: image.placement.height,
                depth_or_array_layers: 1,
            },
        );

        Ok((did_grow, glyph))
    }
}

/// Caches glyphs and images for text rendering.
pub struct TextAtlas {
    /// The cache that maps cache keys to cached glyphs.
    empty_glyphs: hashbrown::HashMap<cosmic_text::CacheKey, GlyphInfo, foldhash::fast::FixedState>,
    /// The atlas that contains color information.
    color_atlas: Atlas,
    /// The atlas that contains mask information.
    mask_atlas: Atlas,

    /// The sampler used to sample from the atlas textures.
    sampler: wgpu::Sampler,
    /// The bind group layout used to create `bind_group`.
    bind_group_layout: wgpu::BindGroupLayout,
    /// The bind group that references both the mask and the color atlas.
    bind_group: wgpu::BindGroup,
}

impl TextAtlas {
    /// Creates a new [`TextAtlas`] from the provided device.
    pub fn new(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI TextAtlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI TextAtlas BindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let color_atlas = Atlas::new(device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mask_atlas = Atlas::new(device, wgpu::TextureFormat::R8Unorm);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI TextAtlas BindGroup"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&color_atlas.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&mask_atlas.texture_view),
                },
            ],
        });

        Self {
            empty_glyphs: Default::default(),
            color_atlas,
            mask_atlas,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }

    /// Inserts the provided image in the cache.
    ///
    /// # Returns
    ///
    /// This function returns the rectangle in which the image was stored.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: cosmic_text::CacheKey,
        image: &Image,
    ) -> Result<GlyphInfo, GlyphError> {
        use swash::scale::image::Content;

        if image.placement.width == 0 || image.placement.height == 0 {
            let info = GlyphInfo {
                atlas_rect: etagere::Rectangle::from_origin_and_size(
                    etagere::Point::new(0, 0),
                    etagere::Size::new(0, 0),
                ),
                placement: image.placement,
                content: image.content,
            };

            self.empty_glyphs.insert(key, info);

            return Ok(info);
        }

        let atlas = match image.content {
            Content::Mask | Content::SubpixelMask => &mut self.mask_atlas,
            Content::Color => &mut self.color_atlas,
        };

        let (did_grow, glyph) = atlas.insert_image(device, queue, key, image)?;

        if did_grow {
            self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI TextAtlas BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &self.color_atlas.texture_view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.mask_atlas.texture_view),
                    },
                ],
            });
        }

        Ok(glyph)
    }

    /// Attempts to get the rectangle in which the glyph is stored.
    ///
    /// If not found, the provided closure is called to rasterize the glyph.
    pub fn get_or_insert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: cosmic_text::CacheKey,
        rasterize: impl FnOnce() -> Result<Image, GlyphError>,
    ) -> Result<GlyphInfo, GlyphError> {
        if let Some(glyph) = self.empty_glyphs.get(&key) {
            return Ok(*glyph);
        } else if let Some(glyph) = self.color_atlas.content.get_mut(&key) {
            glyph.used = true;
            return Ok(glyph.info);
        } else if let Some(glyph) = self.mask_atlas.content.get_mut(&key) {
            glyph.used = true;
            return Ok(glyph.info);
        }

        // We need to rasterize the glyph.
        let image = rasterize()?;
        self.insert_image(device, queue, key, &image)
    }

    /// Marks all glyphs as unused, making them eligible for eviction.
    pub fn trim(&mut self) {
        self.color_atlas.trim();
        self.mask_atlas.trim();
    }

    /// Returns the bind group that references both the mask and the color atlas.
    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the bind group layout used to create `bind_group`.
    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}
