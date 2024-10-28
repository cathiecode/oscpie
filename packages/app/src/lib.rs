use anyhow::Result;
use std::{
    f64::consts::PI,
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};

use oscpie_render::{
    self,
    sprite_renderer::{
        self,
        render_graph::{RenderGraph, RenderPass, Sprite},
        Mat3x3, Vec2,
    },
};

pub struct App<T>
where
    T: sprite_renderer::RenderingBackend,
{
    texture: T::Texture,
    static_pass: Rc<RenderPass<T>>,
}

impl<T> App<T>
where
    T: sprite_renderer::RenderingBackend,
{
    pub fn new(sprite_sheet: T::Texture, backend: &mut T) -> Result<Self> {
        let mut sprites = Vec::<Sprite>::new();

        for _ in 0..100 {
            sprites.push(Sprite {
                matrix: Mat3x3::ideal()
                    .rotate(rand::random::<f32>() * std::f32::consts::PI)
                    .offset(
                        rand::random::<f32>() * 2.0 - 1.0,
                        rand::random::<f32>() * 2.0 - 1.0,
                    ),
            });
        }

        let static_pass = Rc::new(RenderPass {
            texture: sprite_sheet.clone(),
            sprites,
        });

        Ok(Self {
            texture: sprite_sheet,
            static_pass,
        })
    }
}

impl<T> oscpie_render::sprite_renderer::EventHandler<T> for App<T>
where
    T: sprite_renderer::RenderingBackend,
{
    fn draw(
        &mut self,
        backend: &mut T,
    ) -> std::rc::Rc<sprite_renderer::render_graph::RenderGraph<T>> {
        let mut sprites = Vec::<Sprite>::new();

        sprites.push(Sprite {
            matrix: Mat3x3::ideal().offset(
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 2.0 - 1.0,
            ),
        });

        let dynamic_pass = Rc::new(RenderPass {
            texture: self.texture.clone(),
            sprites,
        });

        let rotate = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            .fract()
            * PI
            * 2.0;

        Rc::new(RenderGraph {
            pass: vec![
                self.static_pass.clone(),
                dynamic_pass,
                Rc::new(sprite_renderer::render_graph::RenderPass {
                    texture: self.texture.clone(),
                    sprites: vec![Sprite {
                        matrix: Mat3x3::ideal().scale(1.0, 1.0).rotate(rotate as f32),
                    }],
                }),
            ],
        })
    }
}
