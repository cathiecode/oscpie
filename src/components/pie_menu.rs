use tiny_skia::{Pixmap, Rect, Transform};

use crate::{component::Component, prelude::*, resource::get_sprite_sheet};

use super::pie_menu_item;

pub struct Props {
    pie_menu_input: PieMenuInput,
}

impl Props {
    pub fn new(pie_menu_input: PieMenuInput) -> Self {
        Props { pie_menu_input }
    }
}

pub struct PieMenuComponent {
    center_x: f32,
    center_y: f32,
    radius: f32,
    items: Vec<pie_menu_item::PieMenuItemComponent>,
    input_angle: f32,
    input_magnitude: f32,
}

impl PieMenuComponent {
    pub fn new(center_x: f32, center_y: f32, radius: f32, menu: Menu) -> Self {
        let item_count = menu.items.len();

        let items = menu
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let start_angle = (i as f32 / item_count as f32) * 2.0 * std::f32::consts::PI;
                let end_angle = ((i + 1) as f32 / item_count as f32) * 2.0 * std::f32::consts::PI;

                pie_menu_item::PieMenuItemComponent::new(
                    center_x,
                    center_y,
                    radius,
                    start_angle,
                    end_angle,
                    item.action().clone(),
                    item.icon()
                        .map(|icon_sprite_id| get_sprite_sheet().cutout(icon_sprite_id).unwrap()), // FIXME: Not testable
                )
            })
            .collect();

        Self {
            center_x,
            center_y,
            radius,
            items,
            input_angle: 0.0,
            input_magnitude: 0.0,
        }
    }

    pub fn update(&mut self, props: &Props) {
        self.input_angle = props.pie_menu_input.angle;
        self.input_magnitude = props.pie_menu_input.magnitude;

        for item in &mut self.items {
            item.update(&pie_menu_item::Props::new(&props.pie_menu_input));
        }
    }

    pub fn render(&self, pixmap: &mut Pixmap) {
        // Background
        {
            let mut paint = default_paint();
            paint.set_color(tiny_skia::Color::from_rgba(0.1, 0.1, 0.2, 0.8).unwrap());

            let path =
                tiny_skia::PathBuilder::from_circle(self.center_x, self.center_y, self.radius)
                    .unwrap();

            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::from_translate(0.0, 0.0),
                None,
            );
        }

        // Items
        for item in &self.items {
            item.render(pixmap);
        }

        // Center
        {
            let mut paint = default_paint();
            paint.set_color(tiny_skia::Color::from_rgba(0.1, 0.1, 0.2, 1.0).unwrap());

            let path = tiny_skia::PathBuilder::from_circle(
                self.center_x,
                self.center_y,
                self.radius * 0.3,
            )
            .unwrap();

            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::from_translate(0.0, 0.0),
                None,
            );
        }

        // Stick
        {
            let mut paint = default_paint();
            paint.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 1.0).unwrap());

            let x = self.input_angle.cos() * self.input_magnitude * self.radius;
            let y = self.input_angle.sin() * self.input_magnitude * self.radius;

            pixmap.fill_rect(
                Rect::from_ltrb(x - 10.0, y - 10.0, x + 10.0, y + 10.0).unwrap(),
                &paint,
                Transform::from_translate(self.center_x, self.center_y),
                None,
            );
        }
    }
}

#[cfg(test)]
mod stories {
    pub use super::*;
    pub use crate::prelude::*;
    use crate::story::story;

    fn pie_menu() -> PieMenuComponent {
        let center_x = 256.0;
        let center_y = 256.0;
        let radius = 256.0 * 0.9;

        let mut icon = Pixmap::new(128, 128).unwrap();
        icon.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));

        let menu = Menu {
            items: vec![
                MenuItem::new(MenuItemAction::Noop, None),
                MenuItem::new(MenuItemAction::Noop, None),
                MenuItem::new(MenuItemAction::Noop, None),
                MenuItem::new(MenuItemAction::Noop, None),
            ],
        };

        PieMenuComponent::new(center_x, center_y, radius, menu)
    }

    #[test]
    fn story_pie_menu() {
        story("pie_menu", |pixmap| {
            let mut pie_menu = pie_menu();
            pie_menu.update(&Props::new(PieMenuInput::new(0.1, 1.0, 0.0)));
            pie_menu.render(pixmap);
        });
    }

    #[test]
    fn story_pie_menu_hover() {
        story("pie_menu_hover", |pixmap| {
            let mut pie_menu = pie_menu();
            pie_menu.update(&Props::new(PieMenuInput::new(0.1, 1.0, 0.0)));
            pie_menu.render(pixmap);
        });
    }

    #[test]
    fn story_pie_menu_click() {
        story("pie_menu_click", |pixmap| {
            let mut pie_menu = pie_menu();
            pie_menu.update(&Props::new(PieMenuInput::new(0.1, 1.0, 1.0)));
            pie_menu.render(pixmap);
        });
    }
}
