# wgpu-tilemap

```rust
// Create a tilemap pipeline
let mut tilemap_pipeline = TilemapPipeline::new(device, surface_config.format, None);

// Specify that a camera
tilemap_pipeline.set_camera(queue, FULLSCREEN_QUAD_CAMERA);

// Upload a tileset
tilemap_pipeline.upload_tilesets(
	device,
	queue,
	&[TilesetRef {
		pixel_size: Vec2::new(1, 2),
		size_of_tile: Vec2::new(1, 1),
		data: &[0xffffffff, 0x000000ff],
	}],
);

// Upload a tilemap
self.tilemap_pipeline.upload_tilemaps(
	device,
	queue,
	&[TilemapDrawData {
		transform: Mat4::identity(),
		tilemap: &some_tilemap,
		tileset: 0,
		noise: TilemapNoise::default(),
	}],
);

// Render the uploaded tilemaps
tilemap_pipeline.render(&device, &mut rpass);
```
