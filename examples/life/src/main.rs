use std::{
    borrow::Cow,
    time::{Duration, Instant},
};
use vek::{Mat4, Vec2};
use wgpu_example::framework::Spawner;
use wgpu_tilemap::{TilemapDrawData, TilemapNoise, TilemapPipeline, TilemapRef, TilesetRef};

const TARGET_FRAME_TIME: Duration = Duration::from_millis(16);
const SIDELENGTH: usize = 600;

struct Example {
    state: TilemapRef<'static>,
    last_step: Instant,
    tilemap_pipeline: TilemapPipeline,
}

fn get_pixel(tilemap: &TilemapRef<'_>, x: usize, y: usize) -> u8 {
    tilemap.data.as_ref()[SIDELENGTH * y + x]
}
impl Example {
    fn put_pixel(&mut self, x: usize, y: usize, val: u8) {
        self.state.data.to_mut()[SIDELENGTH * y + x] = val;
    }

    fn step(&mut self) {
        let prev = TilemapRef {
            tile_size: self.state.tile_size,
            data: self.state.data.to_mut().clone().into(),
        };
        for y in 0..SIDELENGTH {
            for x in 0..SIDELENGTH {
                let mut count = 0;
                let mut center = false;
                for dy in 0..=2 {
                    for dx in 0..=2 {
                        let probe_x = (x + SIDELENGTH + dx - 1) % SIDELENGTH;
                        let probe_y = (y + SIDELENGTH + dy - 1) % SIDELENGTH;
                        let current = get_pixel(&prev, probe_x, probe_y) != 0;
                        if dx == 1 && dy == 1 {
                            center = current;
                        } else {
                            count += if current { 1 } else { 0 };
                        }
                    }
                }
                if center && ([2, 3].contains(&count)) {
                    self.put_pixel(x, y, 1);
                } else if !center && [3].contains(&count) {
                    self.put_pixel(x, y, 1);
                } else {
                    self.put_pixel(x, y, 0);
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
        let state = TilemapRef {
            tile_size: Vec2::broadcast(SIDELENGTH).as_::<u32>(),
            data: Cow::from(vec![0; SIDELENGTH * SIDELENGTH]),
        };
        let mut tilemap_pipeline = TilemapPipeline::new(device, config.format, None);
        tilemap_pipeline.set_camera(queue, wgpu_tilemap::FULLSCREEN_QUAD_CAMERA);
        tilemap_pipeline.upload_tilesets(
            device,
            queue,
            &[TilesetRef {
                pixel_size: Vec2::new(1, 2),
                size_of_tile: Vec2::new(1, 1),
                data: &[0xffffffff, 0x000000ff],
            }],
        );
        let mut ret = Example {
            state,
            last_step: Instant::now(),
            tilemap_pipeline,
        };
        // block
        ret.put_pixel(25, 25, 1);
        ret.put_pixel(25, 26, 1);
        ret.put_pixel(26, 25, 1);
        ret.put_pixel(26, 26, 1);
        // glider
        ret.put_pixel(50, 50, 1);
        ret.put_pixel(49, 51, 1);
        ret.put_pixel(49, 52, 1);
        ret.put_pixel(50, 52, 1);
        ret.put_pixel(51, 52, 1);
        // R-pentomino
        ret.put_pixel(301, 300, 1);
        ret.put_pixel(302, 300, 1);
        ret.put_pixel(300, 301, 1);
        ret.put_pixel(301, 301, 1);
        ret.put_pixel(301, 302, 1);
        ret
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
                tilemap: &self.state,
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
