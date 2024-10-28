use std::{ops::Mul, rc::Rc};

use render_graph::RenderGraph;

pub mod miniquad_backend;
pub mod render_graph;

pub type Component = f32;

// Vertical vector
#[derive(Debug, Clone, Copy)]
pub struct Vec2(pub Component, pub Component);

impl Vec2 {
    pub fn length_sq(self) -> Component {
        self.0.powf(2.0) + self.1.powf(2.0)
    }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl std::ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vec3(pub Component, pub Component, pub Component);

// Vertical vector, Vertical vector
#[derive(Debug, Clone)]
pub struct Mat3x3(pub Vec3, pub Vec3, pub Vec3);

impl Mat3x3 {
    pub fn ideal() -> Self {
        Self(Vec3(1.0, 0.0, 0.0), Vec3(0.0, 1.0, 0.0), Vec3(0.0, 0.0, 1.0))
    }

    pub fn transposed(&self) -> Self {
        Self (
            Vec3(
                self.0.0,
                self.1.0,
                self.2.0,
            ),
            Vec3(
                self.0.1,
                self.1.1,
                self.2.1
            ),
            Vec3(
                self.0.2,
                self.1.2,
                self.2.2
            )
        )
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.2.0 += x;
        self.2.1 += y;

        self
    }

    pub fn scale(mut self, w: f32, h: f32) -> Self {
        self.0.0 *= w;
        self.1.1 *= h;

        self
    }

    pub fn rotate(mut self, rad: f32) -> Self {
        let cos = rad.cos();
        let sin = rad.sin();
        
        self = Mat3x3(Vec3(cos, -sin, 1.0), Vec3(sin, cos, 1.0), Vec3(0.0, 0.0, 1.0)) * self;

        self
    }
}

impl std::ops::Mul<Vec2> for &Mat3x3 {
    type Output = Vec2;

    fn mul(self, v: Vec2) -> Self::Output {
        // (selfas std::ops::Mul<Vec3>>).mul(rhs)
        let Vec3(x, y, _) = self * Vec3(v.0, v.1, 1.0);

        Vec2(x, y)
    }
}

impl std::ops::Mul<Vec3> for &Mat3x3 {
    type Output = Vec3;

    fn mul(self, v: Vec3) -> Self::Output {
        Vec3(
            (self.0 .0 * v.0) + (self.1 .0 * v.1) + self.2 .0,
            (self.0 .1 * v.0) + (self.1 .1 * v.1) + self.2 .1,
            (self.0 .2 * v.0) + (self.1 .2 * v.1) + self.2 .2,
        )
    }
}

impl std::ops::Mul<Mat3x3> for Mat3x3 {
    type Output = Mat3x3;

    fn mul(self, rhs: Mat3x3) -> Self::Output {
        Mat3x3(
            Vec3(
                self.0.0 * rhs.0.0 + self.0.1 * rhs.1.0 + self.0.2 * rhs.2.0,
                self.0.0 * rhs.0.1 + self.0.1 * rhs.1.1 + self.0.2 * rhs.2.1,
                self.0.0 * rhs.0.2 + self.0.1 * rhs.1.2 + self.0.2 * rhs.2.2,
            ),
            Vec3(
                self.1.0 * rhs.0.0 + self.1.1 * rhs.1.0 + self.1.2 * rhs.2.0,
                self.1.0 * rhs.0.1 + self.1.1 * rhs.1.1 + self.1.2 * rhs.2.1,
                self.1.0 * rhs.0.2 + self.1.1 * rhs.1.2 + self.1.2 * rhs.2.2,
            ),
            Vec3(
                self.2.0 * rhs.0.0 + self.2.1 * rhs.1.0 + self.2.2 * rhs.2.0,
                self.2.0 * rhs.0.1 + self.2.1 * rhs.1.1 + self.2.2 * rhs.2.1,
                self.2.0 * rhs.0.2 + self.2.1 * rhs.1.2 + self.2.2 * rhs.2.2,
            ),
        )
    }
}

pub trait RenderingBackend {
    type Texture: Clone;
    type Sprite;

    fn texture_from_raw_rgba_u8(&mut self, width: u16, height: u16, data: &[u8]) -> Self::Texture;
    fn get_stage_size(&self) -> Vec2;
}

pub struct NullRenderingBackend;

impl RenderingBackend for NullRenderingBackend {
    type Texture = ();
    type Sprite = ();

    fn texture_from_raw_rgba_u8(&mut self, width: u16, height: u16, data: &[u8]) -> Self::Texture {}
    
    fn get_stage_size(&self) -> Vec2 {
        Vec2(0.0, 0.0)
    }
}

pub trait EventHandler<T>
where
    T: RenderingBackend,
{
    fn update(&mut self) -> bool {
        return false;
    }

    fn draw(&mut self, backend: &mut T) -> Rc<RenderGraph<T>>;
}

pub struct NullEventHandler<T>
where
    T: RenderingBackend,
{
    texture: T::Texture,
}

impl<T> EventHandler<T> for NullEventHandler<T>
where
    T: RenderingBackend,
{
    fn draw(&mut self, backend: &mut T) -> Rc<RenderGraph<T>> {
        self.texture = backend.texture_from_raw_rgba_u8(16, 16, &[0]);

        return Rc::new(RenderGraph {
            pass: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::*;

    const EPS: f32 = 0.001;

    fn assert_f32(left: f32, right: f32) {
        assert_lt!((left - right).abs(), EPS);
    }

    #[test]
    fn mat33_mul_vec2_ideal() {
        let result: Vec2 = &Mat3x3::ideal() * Vec2(1.0, 2.0);
        assert_f32(result.0, 1.0);
        assert_f32(result.1, 2.0);
    }

    #[test]
    fn mat33_mul_vec2_flip_axis() {
        let result: Vec2 = &Mat3x3(Vec3(0.0, 1.0, 0.0), Vec3(1.0, 0.0, 0.0), Vec3(0.0, 0.0, 0.0)) * Vec2(1.0, 2.0);

        assert_f32(result.0, 2.0);
        assert_f32(result.1, 1.0);
    }

    #[test]
    fn mat33_mul_vec2_offset() {
        let result: Vec2 = &Mat3x3(Vec3(1.0, 0.0, 0.0), Vec3(0.0, 1.0, 0.0), Vec3(1.0, 1.0, 0.0)) * Vec2(1.0, 2.0);

        assert_f32(result.0, 2.0);
        assert_f32(result.1, 3.0);
    }

    #[test]
    fn mat33_transposed() {
        let target = Mat3x3(Vec3(0., 1., 2.), Vec3(3., 4., 5.), Vec3(6., 7., 8.));
        
        let result = target.transposed();
        
        assert_f32(target.0.0, result.0.0);
        assert_f32(target.0.1, result.1.0);
        assert_f32(target.0.2, result.2.0);
        assert_f32(target.1.0, result.0.1);
        assert_f32(target.1.1, result.1.1);
        assert_f32(target.1.2, result.2.1);
        assert_f32(target.2.0, result.0.2);
        assert_f32(target.2.1, result.1.2);
        assert_f32(target.2.2, result.2.2);
    }

    #[test]
    fn mat33_mul_ideal() {
        let target_left = Mat3x3(Vec3(0., 1., 2.), Vec3(3., 4., 5.), Vec3(6., 7., 8.));
        let target_right = Mat3x3::ideal();

        let result = target_left.clone() * target_right.clone();

        assert_f32(target_left.0.0, result.0.0);
        assert_f32(target_left.0.1, result.0.1);
        assert_f32(target_left.0.2, result.0.2);

        assert_f32(target_left.1.0, result.1.0);
        assert_f32(target_left.1.1, result.1.1);
        assert_f32(target_left.1.2, result.1.2);

        assert_f32(target_left.2.0, result.2.0);
        assert_f32(target_left.2.1, result.2.1);
        assert_f32(target_left.2.2, result.2.2);
    }

    #[test]
    fn mat33_scale() {
        let result: Vec2 = &Mat3x3::ideal().scale(10.0, 20.0) * Vec2(1.0, 2.0);

        assert_f32(result.0, 10.0);
        assert_f32(result.1, 40.0);
    }

    #[test]
    fn mat33_offset() {
        let result: Vec2 = &Mat3x3::ideal().offset(10.0, 20.0) * Vec2(1.0, 2.0);

        assert_f32(result.0, 11.0);
        assert_f32(result.1, 22.0);
    }
}
