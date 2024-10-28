use std::env::args;

use image::ImageReader;
use oscpie_app::App;
use oscpie_render::sprite_renderer::{self, RenderingBackend};

fn main() {
    miniquad::start(miniquad::conf::Conf::default(), move || {
        let sprite_sheet_image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageReader::open(&args().collect::<Vec<String>>()[1])
            .unwrap()
            .decode()
            .unwrap()
            .resize_exact(1024, 1024, image::imageops::FilterType::Lanczos3)
            .into_rgba8();

        let sprite_sheet_image_data = sprite_sheet_image.as_raw().as_slice();

        Box::new(
            sprite_renderer::miniquad_backend::MiniquadBackendEventHandler::new(|backend| {
                let sprite_sheet =
                    backend.texture_from_raw_rgba_u8(1024, 1024, sprite_sheet_image_data);
                App::new(sprite_sheet, backend).unwrap()
            }),
        )
    });
}
