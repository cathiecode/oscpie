use std::{
    f64::consts::PI,
    rc::Rc,
    time::{self, SystemTime},
};

use sprite_renderer::{
    render_graph::{RenderGraph, Sprite},
    Mat3x3,
};

pub mod sprite_renderer;

struct HelloWorld<T>
where
    T: sprite_renderer::RenderingBackend,
{
    texture: T::Texture,
}

impl<T> HelloWorld<T>
where
    T: sprite_renderer::RenderingBackend,
{
    fn new(backend: &mut T) -> Self {
        let pixels: [u8; 4 * 4 * 4] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
            0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        Self {
            texture: backend.texture_from_raw_rgba_u8(4, 4, &pixels),
        }
    }
}

impl<T> sprite_renderer::EventHandler<T> for HelloWorld<T>
where
    T: sprite_renderer::RenderingBackend,
{
    fn draw(&mut self, backend: &mut T) -> Rc<sprite_renderer::render_graph::RenderGraph<T>> {
        let rotate = std::time::SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            .fract()
            * PI;

        Rc::new(RenderGraph {
            pass: vec![Rc::new(sprite_renderer::render_graph::RenderPass {
                texture: self.texture.clone(),
                sprites: vec![Sprite {
                    matrix: Mat3x3::ideal().scale(1.0, 1.0).rotate(rotate as f32),
                }],
            })],
        })
    }
}

fn main() {
    miniquad::start(miniquad::conf::Conf::default(), move || {
        Box::new(
            sprite_renderer::miniquad_backend::MiniquadBackendEventHandler::new(|backend| {
                HelloWorld::new(backend)
            }),
        )
    });
}
