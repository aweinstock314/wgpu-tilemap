use std::{borrow::Cow, collections::HashMap, hash::Hash, num::NonZeroU64};
use vek::{Mat4, Vec2};

/// Apply noise to the tilemap at a multiple of the tile size (e.g. for sand effects).
/// TilemapNoise::default() applies no noise.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TilemapNoise {
    /// How much noise to apply.
    pub magnitude: f32,
    /// Number of noise cells per tile.
    pub resolution: u8,
}

impl Default for TilemapNoise {
    fn default() -> TilemapNoise {
        TilemapNoise { magnitude: 0.0, resolution: 1 }
    }
}

/// A reference to tilemap data to be uploaded as a texture and used as indices into the tileset.
#[derive(Copy, Clone, Debug)]
pub struct TilemapRef<'a> {
    /// Size of this tilemap, in tiles.
    pub tile_size: Vec2<u32>,
    /// Assumes a maximum of 256 tiles per tileset, represented as `wgpu::TextureFormat::R8Uint`.
    pub data: &'a [u8],
}

/// A reference to tileset data to be uploaded as a texture. This is the image data drawn for each
/// tile of the corresponding tilemap.
#[derive(Copy, Clone, Debug)]
pub struct TilesetRef<'a> {
    /// Size of this tileset, in pixels.
    pub pixel_size: Vec2<u32>,
    /// Size of each tile in this tileset.
    pub size_of_tile: Vec2<u32>,
    /// Interpreted as `wgpu::TextureFormat::Rgba8UnormSrgb`
    pub data: &'a [u32],
}

/// An instruction to draw a tilemap.
#[derive(Copy, Clone, Debug)]
pub struct TilemapDrawData<'a> {
    /// A matrix that maps from [0, 1]x[0, 1] to world coordinates for this tilemap.
    pub transform: Mat4<f32>,
    /// The data to be used for this tilemap.
    pub tilemap: TilemapRef<'a>,
    /// The index into the array of tilesets last provided to the most recent `TilemapPipeline::upload_tilesets` call that this tilemap should be drawn with.
    pub tileset: u32,
    /// How much noise this tilemap should be drawn with.
    pub noise: TilemapNoise,
}

const VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: 0,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[],
};

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct TilesetBuffer {
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
}
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct TilemapBuffer {
    transform: [[f32; 4]; 4],
    width: u32,
    height: u32,
    noise_data: u32,
    _pad: u32,
}

trait HasTextureAllocation {
    type Params: bytemuck::Pod;
    fn active(&self) -> bool;
    fn set_active(&mut self, active: bool);
    fn params_buffer(&self) -> &wgpu::Buffer;
    fn texture(&self) -> &wgpu::Texture;
}

struct FirstFitTextureAllocator<K, T> {
    map: HashMap<K, Vec<T>>,
}

impl<K: Clone + Eq + Hash, T: HasTextureAllocation> FirstFitTextureAllocator<K, T> {
    fn new() -> Self {
        FirstFitTextureAllocator { map: HashMap::new() }
    }

    fn mark_inactive(&mut self) {
        for (_size, data) in self.map.iter_mut() {
            for datum in data.iter_mut() {
                datum.set_active(false);
            }
        }
    }

    fn allocate_and_upload<F, G>(&mut self, size: K, device: &wgpu::Device, queue: &wgpu::Queue, alloc: F, params: &T::Params, callback: G)
    where
        F: FnOnce(&wgpu::Device, K) -> T,
        G: FnOnce(usize, &mut T),
    {
        // Find the first inactive allocation of the correct size, or call the provided allocator if none exists.
        let data = self.map.entry(size.clone()).or_insert_with(Vec::new);
        let (i, datum) = if let Some((i, datum)) = data.iter_mut().enumerate().find(|(_, datum)| !datum.active()) {
            (i, datum)
        } else {
            let i = data.len();
            data.push(alloc(device, size));
            (i, data.last_mut().unwrap())
        };

        // Mark the allocation as active, and let the caller store an index to it.
        datum.set_active(true);
        callback(i, datum);

        // Upload the parameters and texture data for it to the GPU.
        queue.write_buffer(datum.params_buffer(), 0, &bytemuck::bytes_of(params)[..]);
    }
}

/// The entry point to this crate.
pub struct TilemapPipeline {
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    tileset_bind_group_layout: wgpu::BindGroupLayout,
    tilemap_bind_group_layout: wgpu::BindGroupLayout,
    tilemap_pipeline: wgpu::RenderPipeline,
    draw_calls: FirstFitTextureAllocator<Vec2<u32>, TilemapDrawCall>,
    tilesets: FirstFitTextureAllocator<(Vec2<u32>, Vec2<u32>), TilesetCache>,
    active_tilesets: Vec<((Vec2<u32>, Vec2<u32>), u32)>,
}

struct TilemapDrawCall {
    params_buffer: wgpu::Buffer,
    index_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    tilesets_index: ((Vec2<u32>, Vec2<u32>), u32),
    active: bool,
}

struct TilesetCache {
    params_buffer: wgpu::Buffer,
    data_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    active: bool,
}

impl HasTextureAllocation for TilemapDrawCall {
    type Params = TilemapBuffer;
    fn active(&self) -> bool {
        self.active
    }
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    fn params_buffer(&self) -> &wgpu::Buffer {
        &self.params_buffer
    }
    fn texture(&self) -> &wgpu::Texture {
        &self.index_texture
    }
}

impl HasTextureAllocation for TilesetCache {
    type Params = TilesetBuffer;
    fn active(&self) -> bool {
        self.active
    }
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    fn params_buffer(&self) -> &wgpu::Buffer {
        &self.params_buffer
    }
    fn texture(&self) -> &wgpu::Texture {
        &self.data_texture
    }
}

impl TilemapPipeline {
    /// Create a new `TilemapPipeline` capable of rendering to the provided `texture_format`.
    pub fn new(device: &wgpu::Device, texture_format: wgpu::TextureFormat) -> TilemapPipeline {
        let shader_source = Cow::Borrowed(include_str!("tilemap.wgsl"));
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shaders"),
            source: wgpu::ShaderSource::Wgsl(shader_source),
        });
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(::std::mem::size_of::<[[f32; 4]; 4]>() as u64),
                },
                count: None,
            }],
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tilemap_camera_buffer"),
            size: ::std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let tileset_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tileset_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(::std::mem::size_of::<TilesetBuffer>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let tilemap_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tilemap_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(::std::mem::size_of::<TilemapBuffer>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let tilemap_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tilemap_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &tileset_bind_group_layout, &tilemap_bind_group_layout],
            push_constant_ranges: &[],
        });
        let tilemap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tilemap_pipeline"),
            layout: Some(&tilemap_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: &"tilemap_vert_main",
                buffers: &[VERTEX_LAYOUT.clone()],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth16Unorm,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: &"tilemap_frag_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
        let draw_calls = FirstFitTextureAllocator::new();
        let tilesets = FirstFitTextureAllocator::new();
        TilemapPipeline {
            camera_buffer,
            camera_bind_group,
            vertex_buffer,
            tileset_bind_group_layout,
            tilemap_bind_group_layout,
            tilemap_pipeline,
            tilesets,
            active_tilesets: Vec::new(),
            draw_calls,
        }
    }
    fn allocate_tilesets(device: &wgpu::Device, tileset_bind_group_layout: &wgpu::BindGroupLayout, size: Vec2<u32>, tilesize: Vec2<u32>) -> TilesetCache {
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tileset_params_buffer"),
            size: ::std::mem::size_of::<TilesetBuffer>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let data_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tileset_data_texture"),
            //size: wgpu::Extent3d { width: 1368, height: 768, depth_or_array_layers: 1 },
            size: wgpu::Extent3d {
                width: tilesize.x,
                height: tilesize.y,
                depth_or_array_layers: (size.x / tilesize.x) * (size.y / tilesize.y),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let data_view = data_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tileset_bind_group"),
            layout: tileset_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&data_view),
                },
            ],
        });
        TilesetCache {
            params_buffer,
            data_texture,
            bind_group,
            active: false,
        }
    }

    /// Upload a list of tilesets to the GPU, replacing the previous set of tilesets, and reusing texture allocations if the sizes are compatible.
    pub fn upload_tilesets(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, tilesets: &[TilesetRef]) {
        self.active_tilesets.clear();
        self.tilesets.mark_inactive();
        for tileset in tilesets.iter() {
            let params = TilesetBuffer {
                width: tileset.pixel_size.x,
                height: tileset.pixel_size.y,
                tile_width: tileset.size_of_tile.x,
                tile_height: tileset.size_of_tile.y,
            };

            let tile_size = tileset.pixel_size / tileset.size_of_tile;

            self.tilesets.allocate_and_upload(
                (tileset.pixel_size, tileset.size_of_tile),
                device,
                queue,
                |device, (size, tilesize)| TilemapPipeline::allocate_tilesets(device, &self.tileset_bind_group_layout, size, tilesize),
                &params,
                |i, datum| {
                    self.active_tilesets.push(((tileset.pixel_size, tileset.size_of_tile), i as u32));
                    let texture_data = tileset.data;
                    let idl = wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * tileset.size_of_tile.x),
                        rows_per_image: Some(tileset.size_of_tile.y),
                    };
                    let extent = wgpu::Extent3d {
                        width: tileset.size_of_tile.x,
                        height: tileset.size_of_tile.y,
                        depth_or_array_layers: tile_size.x * tile_size.y,
                    };
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &datum.texture(),
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytemuck::cast_slice(&texture_data),
                        idl,
                        extent,
                    );
                },
            );
        }
    }

    /// Upload a list of tilemaps to be drawn this frame. Each tilemap is drawn with an independent
    /// transform and tileset. Texture allocations of matching sizes are reused.
    pub fn upload_tilemaps(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, tilemaps: &[TilemapDrawData]) {
        self.draw_calls.mark_inactive();
        for TilemapDrawData { transform, tilemap, tileset, noise } in tilemaps.iter() {
            let size = tilemap.tile_size;
            let noise_data = ((0xffff as f32 * noise.magnitude) as u32 & 0xffff) | ((noise.resolution as u32 & 0xff) << 16);
            let params = TilemapBuffer {
                transform: transform.into_col_arrays(),
                width: size.x,
                height: size.y,
                noise_data,
                _pad: Default::default(),
            };
            self.draw_calls.allocate_and_upload(
                size,
                device,
                queue,
                |device, size| TilemapPipeline::allocate_draw_call(device, &self.tilemap_bind_group_layout, size),
                &params,
                |_, call| {
                    call.tilesets_index = self.active_tilesets[*tileset as usize];
                    let texture_data = tilemap.data;
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &call.texture(),
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytemuck::cast_slice(&texture_data),
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(size.x),
                            rows_per_image: Some(size.y),
                        },
                        wgpu::Extent3d {
                            width: size.x,
                            height: size.y,
                            depth_or_array_layers: 1,
                        },
                    );
                },
            );
        }
    }

    fn allocate_draw_call(device: &wgpu::Device, tilemap_bind_group_layout: &wgpu::BindGroupLayout, size: Vec2<u32>) -> TilemapDrawCall {
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tilemap_params_buffer"),
            size: ::std::mem::size_of::<TilemapBuffer>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tilemap_index_texture"),
            //size: wgpu::Extent3d { width: 1368, height: 768, depth_or_array_layers: 1 },
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let index_view = index_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tilemap_bind_group"),
            layout: tilemap_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&index_view),
                },
            ],
        });
        TilemapDrawCall {
            params_buffer,
            index_texture,
            bind_group,
            tilesets_index: ((Vec2::zero(), Vec2::zero()), 0),
            active: false,
        }
    }
    /// Set the camera matrix that maps from world coordinates to Normalized Device Coordinates.
    pub fn set_camera(&self, queue: &wgpu::Queue, camera: Mat4<f32>) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera.into_col_arrays()));
    }
    /// Render the tilemaps to the provided renderpass, whose color attachment must match the
    /// texture format provided when this was created.
    pub fn render<'a: 'pass, 'pass>(&'a self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass<'pass>) {
        self.render_with_profiler_inner(device, rpass, &mut ());
    }
    #[cfg(feature = "wgpu-profiler")]
    pub fn render_with_profiler<'a: 'pass, 'pass>(&'a self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass<'pass>, gpu_profiler: &mut wgpu_profiler::GpuProfiler) {
        self.render_with_profiler_inner(device, rpass, gpu_profiler);
    }
    fn render_with_profiler_inner<'a: 'pass, 'pass>(&'a self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass<'pass>, gpu_profiler: &mut impl ProfilerShim) {
        gpu_profiler.begin_scope("tilemap", rpass, device);
        rpass.set_pipeline(&self.tilemap_pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_bind_group(0, &self.camera_bind_group, &[]);

        // TODO: sort/bucket by tileset to minimize rebinding of the tilesets texture
        for (_sz, calls) in self.draw_calls.map.iter() {
            for call in calls.iter() {
                if call.active {
                    let Some(tilesets_bg) = self.tilesets.map.get(&call.tilesets_index.0).and_then(|v| v.get(call.tilesets_index.1 as usize)) else { continue };
                    gpu_profiler.begin_scope("tilemap_draw", rpass, device);
                    rpass.set_bind_group(1, &tilesets_bg.bind_group, &[]);
                    rpass.set_bind_group(2, &call.bind_group, &[]);
                    rpass.draw(0..6, 0..1);
                    gpu_profiler.end_scope(rpass);
                }
            }
        }
        gpu_profiler.end_scope(rpass);
    }
}

trait ProfilerShim {
    fn begin_scope(&mut self, span: &str, rpass: &mut wgpu::RenderPass, device: &wgpu::Device);
    fn end_scope(&mut self, rpass: &mut wgpu::RenderPass);
}

impl ProfilerShim for () {
    fn begin_scope(&mut self, _span: &str, _rpass: &mut wgpu::RenderPass, _device: &wgpu::Device) {}
    fn end_scope(&mut self, _rpass: &mut wgpu::RenderPass) {}
}

#[cfg(feature = "wgpu-profiler")]
impl ProfilerShim for wgpu_profiler::GpuProfiler {
    fn begin_scope(&mut self, span: &str, rpass: &mut wgpu::RenderPass, device: &wgpu::Device) {
        (*self).begin_scope(span, rpass, device)
    }
    fn end_scope(&mut self, rpass: &mut wgpu::RenderPass) {
        (*self).end_scope(rpass)
    }
}