# wgpu-tilemap

```rust
// Create a tilemap pipeline
let mut tilemap_pipeline = TilemapPipeline::new(device, surface_config.format, None);

// Specify that a camera
tilemap_pipeline.set_camera(queue, FULLSCREEN_QUAD_CAMERA);

// Create/load a tileset.
let tileset = TilesetRef {
	pixel_size: Vec2::new(1, 2),
	size_of_tile: Vec2::new(1, 1),
	data: Cow::Borrowed(&[0xffffffff, 0x000000ff]),
};

// Upload a tileset to the GPU
tilemap_pipeline.upload_tilesets(device, queue, &[tileset]);

// Create/load a tilemap
let some_tilemap = TilemapRef::zeroed(Vec2::broadcast(size));

// Upload a tilemap to the GPU
self.tilemap_pipeline.upload_tilemaps(
	device,
	queue,
	&[TilemapDrawData {
		transform: Mat4::identity(),
		tilemap: Cow::Borrowed(&some_tilemap),
		tileset: 0,
		noise: TilemapNoise::default(),
	}],
);

// Render the uploaded tilemaps
tilemap_pipeline.render(&device, &mut rpass);
```
