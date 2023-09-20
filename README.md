# wgpu-tilemap

`wgpu-tilemap` is [wgpu middleware](https://github.com/gfx-rs/wgpu/wiki/Encapsulating-Graphics-Work#middleware-libraries) for GPU-accelerated tilemap rendering, primarily targeted at 2d games.

It draws each tilemap as a single quad, so the vertex count is independent of the size of the tilemap.
It uses texture arrays for the tilesets, so the fragment shader is essentially 2 texture loads: one from the tilemap and one from the tileset.
It discards fully transparent fragments, so drawing multiple layers can be accelerated with a depth buffer.

## Example

```rust
// Create a tilemap pipeline
let mut tilemap_pipeline = TilemapPipeline::new(device, surface_config.format, None);

// Specify a camera matrix
tilemap_pipeline.set_camera(queue, FULLSCREEN_QUAD_CAMERA);

// Create/load a tileset
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

## License
`wgpu-tilemap` is licensed under the Apache License, Version 2.0, ([LICENSE.apache2](LICENSE.apache2) or <https://www.apache.org/licenses/LICENSE-2.0>)

