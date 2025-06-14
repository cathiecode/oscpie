use tiny_skia::{FilterQuality, Pixmap, PixmapPaint};

pub enum LayoutMode {
    Center,
}

pub struct Props {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub layout_mode: LayoutMode,
}

pub struct SpriteComponent {
    pixmap: Pixmap,
    image_width: u32,
    image_height: u32,
    x: f32,
    y: f32,
    scale_x: f32,
    scale_y: f32,
}

#[allow(clippy::cast_precision_loss)]
impl SpriteComponent {
    pub fn new(pixmap: Pixmap) -> Self {
        let image_width = pixmap.width();
        let image_height = pixmap.height();

        Self {
            pixmap,
            image_width,
            image_height,
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }

    pub fn update(&mut self, props: &Props) {
        (self.x, self.y) = match props.layout_mode {
            LayoutMode::Center => (
                props.x - (props.width / 2.0),
                props.y - (props.height / 2.0),
            ),
        };

        (self.scale_x, self.scale_y) = (
            props.width / self.image_width as f32,
            props.height / self.image_height as f32,
        );
    }

    pub fn render(&self, target: &mut Pixmap) {
        let paint = PixmapPaint {
            quality: FilterQuality::Nearest,
            ..PixmapPaint::default()
        };

        target.draw_pixmap(
            0,
            0,
            self.pixmap.as_ref(),
            &paint,
            tiny_skia::Transform::default()
                .post_scale(self.scale_x, self.scale_y)
                .post_translate(self.x, self.y),
            None,
        );
    }

    pub fn width(&self) -> u32 {
        self.image_width
    }

    pub fn height(&self) -> u32 {
        self.image_height
    }
}

#[cfg(test)]
mod stories {
    use crate::story::story;

    use super::*;
    use tiny_skia::Pixmap;

    #[allow(clippy::cast_precision_loss)]
    #[test]
    fn story_sprite_component() {
        story("sprite", |pixmap| {
            let mut sprite_image = Pixmap::new(128, 128).unwrap();
            sprite_image.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));

            let mut sprite = SpriteComponent::new(sprite_image);

            let props = Props {
                x: pixmap.width() as f32 / 2.0,
                y: pixmap.height() as f32 / 2.0,
                width: pixmap.width() as f32 / 2.0,
                height: pixmap.height() as f32 / 2.0,
                layout_mode: LayoutMode::Center,
            };

            sprite.update(&props);

            sprite.render(pixmap);
        });
    }
}
