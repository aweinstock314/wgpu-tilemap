use std::{
    borrow::Cow,
    time::{Duration, Instant},
};
use vek::{Mat4, Vec2};
use wgpu_example::framework::Spawner;
use wgpu_tilemap::{TilemapDrawData, TilemapNoise, TilemapPipeline, TilemapRef, TilesetRef};

const TARGET_FRAME_TIME: Duration = Duration::from_millis(16);
const SIDELENGTH: u32 = 600;

struct Example {
    state: TilemapRef<'static>,
    last_step: Instant,
    tilemap_pipeline: TilemapPipeline,
}

impl Example {
    fn step(&mut self) {
        let prev = self.state.clone();
        for y in 0..SIDELENGTH {
            for x in 0..SIDELENGTH {
                let mut count = 0;
                let mut center = false;
                for dy in 0..=2 {
                    for dx in 0..=2 {
                        let probe_x = (x + SIDELENGTH + dx - 1) % SIDELENGTH;
                        let probe_y = (y + SIDELENGTH + dy - 1) % SIDELENGTH;
                        let current = prev.get_tile(probe_x, probe_y) != 0;
                        if dx == 1 && dy == 1 {
                            center = current;
                        } else {
                            count += if current { 1 } else { 0 };
                        }
                    }
                }
                if center && ([2, 3].contains(&count)) {
                    self.state.put_tile(x, y, 1);
                } else if !center && [3].contains(&count) {
                    self.state.put_tile(x, y, 1);
                } else {
                    self.state.put_tile(x, y, 0);
                }
            }
        }
    }
}

impl wgpu_example::framework::Example for Example {
    fn init(
        config: &wgpu::SurfaceConfiguration,
        _: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut state = TilemapRef::new_zeroed(Vec2::broadcast(SIDELENGTH));
        let mut tilemap_pipeline = TilemapPipeline::new(device, config.format, None);
        tilemap_pipeline.set_camera(queue, wgpu_tilemap::FULLSCREEN_QUAD_CAMERA);
        tilemap_pipeline.upload_tilesets(
            device,
            queue,
            &[TilesetRef {
                pixel_size: Vec2::new(1, 2),
                size_of_tile: Vec2::new(1, 1),
                data: Cow::Borrowed(&[0xffffffff, 0x000000ff]),
            }],
        );
        // block
        state.put_tile(25, 25, 1);
        state.put_tile(25, 26, 1);
        state.put_tile(26, 25, 1);
        state.put_tile(26, 26, 1);
        // glider
        state.put_tile(50, 50, 1);
        state.put_tile(49, 51, 1);
        state.put_tile(49, 52, 1);
        state.put_tile(50, 52, 1);
        state.put_tile(51, 52, 1);
        // R-pentomino
        state.put_tile(301, 300, 1);
        state.put_tile(302, 300, 1);
        state.put_tile(300, 301, 1);
        state.put_tile(301, 301, 1);
        state.put_tile(301, 302, 1);
        Example {
            state,
            last_step: Instant::now(),
            tilemap_pipeline,
        }
    }
    fn resize(&mut self, _: &wgpu::SurfaceConfiguration, _: &wgpu::Device, _: &wgpu::Queue) {}
    fn update(&mut self, _: winit::event::WindowEvent<'_>) {}
    fn render(
        &mut self,
        surface: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _: &Spawner<'_>,
    ) {
        let now = Instant::now();
        if now - self.last_step >= TARGET_FRAME_TIME {
            self.last_step = now;
            self.step();
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame_encoder"),
        });
        self.tilemap_pipeline.upload_tilemaps(
            device,
            queue,
            &[TilemapDrawData {
                transform: Mat4::identity(),
                tilemap: Cow::Borrowed(&self.state),
                tileset: 0,
                noise: TilemapNoise::default(),
            }],
        );
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("surface_rpass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            self.tilemap_pipeline.render(&device, &mut rpass);
        }
        queue.submit(vec![encoder.finish()]);
    }
}

fn main() {
    wgpu_example::framework::run::<Example>("life")
}
