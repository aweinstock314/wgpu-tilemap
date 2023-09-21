use std::{
    borrow::Cow,
    fs::File,
};
use vek::{Mat4, Vec2};
use wgpu_example::framework::Spawner;
use wgpu_tilemap::{TilemapDrawData, TilemapNoise, TilemapPipeline, TilemapRef, TilesetRef};

const SIDELENGTH: u32 = 30;

struct Example {
    state: TilemapRef<'static>,
    tilemap_pipeline: TilemapPipeline,
}

impl wgpu_example::framework::Example for Example {
    fn init(
        config: &wgpu::SurfaceConfiguration,
        _: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut tilemap_pipeline = TilemapPipeline::new(device, config.format, None);
        use image::io::Reader as ImageReader;
        let image = ImageReader::open("tiles_spritesheet.png")
            .unwrap()
            .decode()
            .unwrap();
        tilemap_pipeline.set_camera(queue, wgpu_tilemap::FULLSCREEN_QUAD_CAMERA);
        let tileset = TilesetRef::from_image_with_spacing(&image, Vec2::broadcast(70), Vec2::broadcast(2));
        tilemap_pipeline.upload_tilesets(device, queue, &[tileset]);
        let csv = File::open("example_tilemap.csv").unwrap();
        let tilemap = TilemapRef::from_csv(Vec2::broadcast(SIDELENGTH), csv).unwrap();
        Example {
            state: tilemap,
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
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
    wgpu_example::framework::run::<Example>("png_tileset")
}
