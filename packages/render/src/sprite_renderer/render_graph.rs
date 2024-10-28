use std::rc::Rc;

use super::{Mat3x3, RenderingBackend};

pub struct RenderGraph<T> where T: RenderingBackend {
    pub pass: Vec<Rc<RenderPass<T>>>
}

pub struct RenderPass<T> where T: RenderingBackend {
    pub texture: T::Texture,
    pub sprites: Vec<Sprite>
}

pub struct Sprite {
    pub matrix: Mat3x3,
}
