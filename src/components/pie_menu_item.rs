use crate::component::Component;
use crate::prelude::*;
use tiny_skia::{FillRule, Pixmap, Transform};

pub struct Props<'a> {
    pub pie_menu_input: &'a PieMenuInput,
}

impl<'a> Props<'a> {
    pub fn new(pie_menu_input: &'a PieMenuInput) -> Self {
        Props { pie_menu_input }
    }
}

#[derive(Debug, Clone)]
pub enum CallbackProps {
    Action(MenuItemAction),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Highlight {
    None,
    Soft,
    Hard,
}

pub struct PieMenuItemComponent {
    center_x: f32,
    center_y: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    action: MenuItemAction,
    callback: Box<dyn Fn(CallbackProps)>,
    click_started: bool,
    highlight: Highlight,
}

impl PieMenuItemComponent {
    pub fn new(
        center_x: f32,
        center_y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        action: MenuItemAction,
        callback: Box<dyn Fn(CallbackProps)>,
    ) -> Self {
        Self {
            center_x,
            center_y,
            radius,
            start_angle,
            end_angle,
            action,
            callback,
            click_started: false,
            highlight: Highlight::None,
        }
    }
}

impl Component for PieMenuItemComponent {
    type Props<'a> = Props<'a>;

    fn update(&mut self, props: &Props) {
        let input = &props.pie_menu_input;
        let in_angle = self.start_angle <= input.angle && input.angle <= self.end_angle;
        let hover_self = in_angle && input.magnitude > 0.5;
        let clicking = input.click > 0.5 && input.magnitude > 0.5;
        let clicking_self = clicking && in_angle;

        if clicking_self && !self.click_started {
            self.click_started = true;
        }

        if in_angle && !clicking && self.click_started {
            (self.callback)(CallbackProps::Action(self.action.clone()));
        }

        if !clicking {
            self.click_started = false;
        }

        if self.click_started && clicking_self {
            self.highlight = Highlight::Hard;
        } else if hover_self {
            self.highlight = Highlight::Soft;
        } else {
            self.highlight = Highlight::None;
        }
    }
    fn render(&self, pixmap: &mut Pixmap) {
        let transform = Transform::from_translate(self.center_x, self.center_y);

        // Highlight
        {
            let path = {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(0.0, 0.0);
                pb.line_to(
                    self.end_angle.cos() * self.radius,
                    self.end_angle.sin() * self.radius,
                );
                pb.line_to(
                    lerp(self.end_angle, self.start_angle, 0.25).cos() * self.radius,
                    lerp(self.end_angle, self.start_angle, 0.25).sin() * self.radius,
                );

                pb.line_to(
                    lerp(self.end_angle, self.start_angle, 0.5).cos() * self.radius,
                    lerp(self.end_angle, self.start_angle, 0.5).sin() * self.radius,
                );
                pb.line_to(
                    lerp(self.end_angle, self.start_angle, 0.75).cos() * self.radius,
                    lerp(self.end_angle, self.start_angle, 0.75).sin() * self.radius,
                );
                pb.line_to(
                    self.start_angle.cos() * self.radius,
                    self.start_angle.sin() * self.radius,
                );
                pb.line_to(0.0, 0.0);

                pb.finish().unwrap()
            };

            let mut paint = default_paint();

            // Draw the highlight
            match self.highlight {
                Highlight::Soft => {
                    paint.set_color_rgba8(255, 255, 0, 128);
                    pixmap.fill_path(&path, &paint, FillRule::EvenOdd, transform, None);
                }
                Highlight::Hard => {
                    paint.set_color_rgba8(255, 255, 0, 255);
                    pixmap.fill_path(&path, &paint, FillRule::EvenOdd, transform, None);
                }
                _ => {}
            }
        }

        // Separate line
        {
            let path = {
                let mut pb = tiny_skia::PathBuilder::new();

                pb.move_to(
                    self.start_angle.cos() * self.radius * 0.4,
                    self.start_angle.sin() * self.radius * 0.4,
                );

                pb.line_to(
                    self.start_angle.cos() * self.radius * 0.9,
                    self.start_angle.sin() * self.radius * 0.9,
                );

                pb.finish().unwrap()
            };

            let mut paint = default_paint();
            let mut stroke = tiny_skia::Stroke::default();
            stroke.width = 4.0;

            paint.set_color_rgba8(255, 255, 255, 255);
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{cell::RefCell, f32::consts::PI, rc::Rc};

    fn pie_menu_item(callback_variable: Rc<RefCell<u32>>) -> PieMenuItemComponent {
        let start_angle = 0.0; // 0 degrees
        let end_angle = PI * 2.0 * 0.25; // 90 degrees
        let action = MenuItemAction::SubMenu {
            to: MenuId::new("sub_menu".to_string()),
        };

        let callback = Box::new(move |props| match props {
            CallbackProps::Action(_) => {
                *callback_variable.borrow_mut() += 1;
            }
        });

        PieMenuItemComponent::new(0.0, 0.0, 0.0, start_angle, end_angle, action, callback)
    }

    #[test]
    fn test_pie_menu_item() {
        // Test the creation of a PieMenuItemComponent
        let is_action_executed = Rc::new(RefCell::new(0));
        let mut pie_menu_item = pie_menu_item(is_action_executed.clone());

        let neutral_angle = 0f32;
        let hover_angle = PI * 2.0 * 0.125; // 45 degrees
        let unhover_angle = PI * 2.0 * 0.5; // 180 degrees

        // Neutral
        pie_menu_item.update(&Props::new(&PieMenuInput::new(neutral_angle, 0.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Hover
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Unhover
        pie_menu_item.update(&Props::new(&PieMenuInput::new(unhover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Click(unhover)
        pie_menu_item.update(&Props::new(&PieMenuInput::new(unhover_angle, 1.0, 1.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Unclick
        pie_menu_item.update(&Props::new(&PieMenuInput::new(unhover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Hover
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Click(hover)
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 1.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Unhover while click
        pie_menu_item.update(&Props::new(&PieMenuInput::new(unhover_angle, 1.0, 1.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Unclick
        pie_menu_item.update(&Props::new(&PieMenuInput::new(unhover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Hover
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Click(hover)
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 1.0)));
        assert_eq!(*is_action_executed.borrow(), 0);

        // Unclick
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 1);

        // Click(hover)
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 1.0)));
        assert_eq!(*is_action_executed.borrow(), 1);

        // Unclick
        pie_menu_item.update(&Props::new(&PieMenuInput::new(hover_angle, 1.0, 0.0)));
        assert_eq!(*is_action_executed.borrow(), 2);
    }
}

#[cfg(test)]
mod stories {
    // NOTE: Allow unused_imports to import Component trait
    #![allow(unused_imports)]
    pub use crate::component::Component;
    use crate::{story::story, types::PieMenuInput};

    use super::{CallbackProps, MenuId, MenuItemAction, PieMenuItemComponent, Pixmap, Props};
    use std::{cell::RefCell, f32::consts::PI, path::PathBuf, rc::Rc};

    static NEUTRAL_ANGLE: f32 = 0.0;
    static START_ANGLE: f32 = 0.0;
    static HOVER_ANGLE: f32 = PI * 2.0 * 0.125;
    static END_ANGLE: f32 = PI * 2.0 * 0.25;
    static UNHOVER_ANGLE: f32 = PI * 2.0 * 0.5;

    fn pie_menu_item(callback_variable: Rc<RefCell<u32>>) -> PieMenuItemComponent {
        let action = MenuItemAction::SubMenu {
            to: MenuId::new("sub_menu".to_string()),
        };

        let callback = Box::new(move |props| match props {
            CallbackProps::Action(_) => {
                *callback_variable.borrow_mut() += 1;
            }
        });

        PieMenuItemComponent::new(
            256.0,
            256.0,
            256.0,
            START_ANGLE,
            END_ANGLE,
            action,
            callback,
        )
    }

    #[test]
    fn story_pie_menu_item_neutral() {
        story("neutral", |pixmap| {
            let mut pie_menu_item = pie_menu_item(Rc::new(RefCell::new(0)));
            pie_menu_item.update(&Props::new(&PieMenuInput::new(NEUTRAL_ANGLE, 0.0, 0.0)));
            pie_menu_item.render(pixmap);
        });
    }

    #[test]
    fn story_pie_menu_item_hover() {
        story("hover", |pixmap| {
            let mut pie_menu_item = pie_menu_item(Rc::new(RefCell::new(0)));
            pie_menu_item.update(&Props::new(&PieMenuInput::new(HOVER_ANGLE, 1.0, 0.0)));
            pie_menu_item.render(pixmap);
        });
    }

    #[test]
    fn story_pie_menu_item_click() {
        story("click", |pixmap| {
            let mut pie_menu_item = pie_menu_item(Rc::new(RefCell::new(0)));
            pie_menu_item.update(&Props::new(&PieMenuInput::new(HOVER_ANGLE, 1.0, 1.0)));
            pie_menu_item.render(pixmap);
        });
    }
}
