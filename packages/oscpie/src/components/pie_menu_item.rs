use crate::prelude::*;
use crate::resource::get_sprite_sheet;
use crate::{component::Component, debug::rt_debug};
use tiny_skia::{Pixmap, Transform};

use super::sprite::{self, SpriteComponent};

pub struct Props<'a> {
    pub pie_menu_input: &'a PieMenuInput,
}

impl<'a> Props<'a> {
    pub fn new(pie_menu_input: &'a PieMenuInput) -> Self {
        Props { pie_menu_input }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StateMachine {
    Neutral, // To: Hovering, Pressing, PressingStartedInOutOfBounds
    Hovering,
    Pressing,
    PressingStartedButOutOfBounds,
    PressingStartedInOutOfBounds,
    Clicked,
}

impl StateMachine {
    pub fn update(&mut self, is_down: bool, is_hovering_self: bool) {
        *self = match self {
            StateMachine::Neutral => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Hovering,
                (true, false) => StateMachine::PressingStartedInOutOfBounds,
                (true, true) => StateMachine::Pressing,
            },
            StateMachine::Hovering => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Hovering,
                (true, false) => StateMachine::PressingStartedInOutOfBounds,
                (true, true) => StateMachine::Pressing,
            },
            StateMachine::Pressing => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Clicked,
                (true, false) => StateMachine::PressingStartedButOutOfBounds,
                (true, true) => StateMachine::Pressing,
            },
            StateMachine::Clicked => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Hovering,
                (true, false) => StateMachine::PressingStartedInOutOfBounds,
                (true, true) => StateMachine::Pressing,
            },
            StateMachine::PressingStartedButOutOfBounds => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Hovering,
                (true, false) => StateMachine::PressingStartedButOutOfBounds,
                (true, true) => StateMachine::Pressing,
            },
            StateMachine::PressingStartedInOutOfBounds => match (is_down, is_hovering_self) {
                (false, false) => StateMachine::Neutral,
                (false, true) => StateMachine::Hovering,
                (true, false) => StateMachine::PressingStartedInOutOfBounds,
                (true, true) => StateMachine::PressingStartedInOutOfBounds,
            },
        };
    }
}

pub struct PieMenuItemComponent {
    center_x: f32,
    center_y: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    action: MenuItemAction,
    state_machine: StateMachine,
    icon_component: Option<SpriteComponent>,
    icon_size: ExponentialSmoothing<f32>,
    time_delta: TimeDelta,
    spin_icon: SpriteComponent,
    spin_icon_size: ExponentialSmoothing<f32>,
}

impl PieMenuItemComponent {
    pub fn new(
        center_x: f32,
        center_y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        action: MenuItemAction,
        icon: Option<Pixmap>,
    ) -> Self {
        Self {
            center_x,
            center_y,
            radius,
            start_angle,
            end_angle,
            action,
            // callback,
            state_machine: StateMachine::Neutral,
            icon_component: icon.map(SpriteComponent::new),
            icon_size: ExponentialSmoothing::new(0.0, 20.0),
            time_delta: TimeDelta::new(),
            spin_icon: SpriteComponent::new(
                get_sprite_sheet()
                    .map_or(Pixmap::new(1, 1).unwrap(), |ss| ss.cutout("spin").unwrap()),
            ),
            spin_icon_size: ExponentialSmoothing::new(0.0, 10.0),
        }
    }
}

impl Component for PieMenuItemComponent {
    type Props<'a> = Props<'a>;

    #[allow(clippy::cast_possible_truncation)]
    fn update(&mut self, props: &Props) {
        let input = &props.pie_menu_input;
        let in_angle = self.start_angle <= input.angle && input.angle <= self.end_angle;
        let hover_self = in_angle && input.magnitude > 0.5;
        let clicking = input.click > 0.5 && input.magnitude > 0.5;

        self.time_delta.update_and_get_secs();

        self.state_machine.update(clicking, hover_self);

        /*if self.state_machine == StateMachine::Clicked {
            // (self.callback)(CallbackProps::Action(self.action.clone()));
        }

        match &self.action {
            MenuItemAction::Noop => {}
            MenuItemAction::Button(ref button_action) => {}
        }*/

        match &self.action {
            MenuItemAction::Noop => {
                // no op
            }
            MenuItemAction::OneShotButton(behaviour) => {
                if self.state_machine == StateMachine::Clicked {
                    behaviour.borrow_mut().on_change(true);
                }
            }
            MenuItemAction::Button(behaviour) => {
                behaviour
                    .borrow_mut()
                    .on_change(self.state_machine == StateMachine::Pressing);
            }
        }

        let spin_icon_scale = self.spin_icon_size.update(
            match &self.action {
                MenuItemAction::Noop => 0.1,
                MenuItemAction::OneShotButton(behaviour) | MenuItemAction::Button(behaviour) => {
                    if behaviour.borrow().value() {
                        1.0
                    } else {
                        0.0
                    }
                }
            },
            self.time_delta.get_without_update_secs(),
        );

        // rt_debug("50_PieMenuItem State", || format!("{:?}", self.state_machine));

        rt_debug(|| {
            (
                format!("50_PieMenuItem '{:?}' State", self.action),
                format!("{:?}", self.state_machine),
            )
        });

        let icon_size_target = match self.state_machine {
            StateMachine::Hovering => 1.2,
            StateMachine::Pressing => 0.8,
            StateMachine::Clicked => 1.2,
            _ => 1.0,
        };

        self.icon_size
            .update(icon_size_target, self.time_delta.get_without_update_secs());

        let middle_angle = f32::midpoint(self.start_angle, self.end_angle);

        if let Some(icon_component) = &mut self.icon_component {
            icon_component.update(&sprite::Props {
                x: self.center_x + self.radius * 0.7 * middle_angle.cos(),
                y: self.center_y + self.radius * 0.7 * middle_angle.sin(),
                width: self.radius * 0.25 * self.icon_size.get_current(),
                height: self.radius * 0.25 * self.icon_size.get_current(),
                rotate: 0.0,
                layout_mode: sprite::LayoutMode::Center,
            });
        }

        self.spin_icon.update(&sprite::Props {
            x: self.center_x + self.radius * 0.7 * middle_angle.cos(),
            y: self.center_y + self.radius * 0.7 * middle_angle.sin(),
            width: self.radius * 0.4 * spin_icon_scale,
            height: self.radius * 0.4 * spin_icon_scale,
            rotate: ((get_time_since_start_secs_f64() as f32) % 360.0) * (360.0 / 1.0),
            layout_mode: sprite::LayoutMode::Center,
        });
    }
    fn render(&self, pixmap: &mut Pixmap) {
        let transform = Transform::from_translate(self.center_x, self.center_y);

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

        // Icon
        {
            if let Some(icon_component) = &self.icon_component {
                icon_component.render(pixmap);
            }
        }

        // Spin icon
        {
            if self.spin_icon_size.get_current() > 0.01 {
                self.spin_icon.render(pixmap);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{cell::RefCell, f32::consts::PI, rc::Rc};

    #[derive(Debug)]
    struct CountAction {
        count: Rc<RefCell<u32>>,
    }

    impl CountAction {
        fn new(count: Rc<RefCell<u32>>) -> Self {
            CountAction { count }
        }
    }

    impl MenuActionBehaviour<bool> for CountAction {
        fn value(&self) -> bool {
            false
        }

        fn on_change(&mut self, _value: bool) {
            *self.count.borrow_mut() += 1;
        }
    }

    fn pie_menu_item(callback_variable: Rc<RefCell<u32>>) -> PieMenuItemComponent {
        let start_angle = 0.0; // 0 degrees
        let end_angle = PI * 2.0 * 0.25; // 90 degrees
        let action = MenuItemAction::OneShotButton(Rc::new(RefCell::new(CountAction::new(
            callback_variable,
        ))));

        PieMenuItemComponent::new(0.0, 0.0, 0.0, start_angle, end_angle, action, None)
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
    use crate::{
        menu::{MenuActionBehaviour, PieMenuInput},
        resource::SPRITE_SHEET,
        story::story,
    };

    use super::{MenuId, MenuItemAction, PieMenuItemComponent, Pixmap, Props};
    use std::{cell::RefCell, f32::consts::PI, path::PathBuf, rc::Rc};

    static NEUTRAL_ANGLE: f32 = 0.0;
    static START_ANGLE: f32 = 0.0;
    static HOVER_ANGLE: f32 = PI * 2.0 * 0.125;
    static END_ANGLE: f32 = PI * 2.0 * 0.25;
    static UNHOVER_ANGLE: f32 = PI * 2.0 * 0.5;

    #[derive(Debug)]
    struct CountAction {
        count: Rc<RefCell<u32>>,
    }

    impl CountAction {
        fn new(count: Rc<RefCell<u32>>) -> Self {
            CountAction { count }
        }
    }

    impl MenuActionBehaviour<bool> for CountAction {
        fn value(&self) -> bool {
            false
        }

        fn on_change(&mut self, _value: bool) {
            *self.count.borrow_mut() += 1;
        }
    }

    fn pie_menu_item(callback_variable: Rc<RefCell<u32>>) -> PieMenuItemComponent {
        let action = MenuItemAction::OneShotButton(Rc::new(RefCell::new(CountAction::new(
            callback_variable,
        ))));

        let mut icon = Pixmap::new(128, 128).unwrap();
        icon.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));

        PieMenuItemComponent::new(
            256.0,
            256.0,
            256.0,
            START_ANGLE,
            END_ANGLE,
            action,
            Some(icon),
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
