use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use crate::{
    action_behaviours::{exec::ExecOneShotButtonAction, key_stroke::KeyStrokeButtonAction},
    config,
};

#[derive(Debug, Clone)]
pub enum AppEvent {
    PopStack,
    PushStack(MenuId),
}

#[derive(Debug)]
pub struct AppEventMenuActionBehaviour {
    received_events: Rc<RefCell<Vec<AppEvent>>>,
    event: AppEvent,
}

impl AppEventMenuActionBehaviour {
    pub fn new(received_events: Rc<RefCell<Vec<AppEvent>>>, event: AppEvent) -> Self {
        Self {
            received_events,
            event,
        }
    }
}

impl MenuActionBehaviour<bool> for AppEventMenuActionBehaviour {
    fn value(&self) -> bool {
        false
    }

    fn on_change(&mut self, _value: bool) {
        self.received_events.borrow_mut().push(self.event.clone());
    }
}

pub trait MenuActionBehaviour<T>: Debug {
    fn value(&self) -> T;
    fn on_change(&mut self, value: T);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuId(String);

impl MenuId {
    pub fn new(id: String) -> Self {
        MenuId(id.to_string())
    }
}

impl MenuId {
    pub fn from_config(id: &config::types::MenuId) -> Self {
        MenuId(id.inner().to_string())
    }
}

#[derive(Debug, Clone)]
pub enum MenuItemAction {
    Noop,
    OneShotButton(Rc<RefCell<dyn MenuActionBehaviour<bool>>>),
    Button(Rc<RefCell<dyn MenuActionBehaviour<bool>>>),
}

impl MenuItemAction {
    pub fn from_config(
        action: &config::types::MenuItemAction,
        app_received_events: Rc<RefCell<Vec<AppEvent>>>,
    ) -> MenuItemAction {
        match action {
            config::types::MenuItemAction::SubMenu { to } => MenuItemAction::OneShotButton(
                Rc::new(RefCell::new(AppEventMenuActionBehaviour::new(
                    app_received_events,
                    AppEvent::PushStack(MenuId::from_config(to)),
                ))),
            ),
            config::types::MenuItemAction::KeyStroke { key_stroke } => {
                MenuItemAction::OneShotButton(Rc::new(RefCell::new(KeyStrokeButtonAction::new(
                    key_stroke.clone().into(),
                ))))
            }
            config::types::MenuItemAction::Exec { program_path, args } => {
                MenuItemAction::OneShotButton(Rc::new(RefCell::new(ExecOneShotButtonAction::new(
                    program_path.clone(),
                    args.clone(),
                ))))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    action: MenuItemAction,
    icon: Option<String>,
}

impl MenuItem {
    pub fn new(action: MenuItemAction, icon: Option<String>) -> Self {
        MenuItem { action, icon }
    }

    pub fn from_config(
        item: &config::types::MenuItem,
        app_received_events: Rc<RefCell<Vec<AppEvent>>>,
    ) -> Self {
        MenuItem {
            action: MenuItemAction::from_config(&item.action, app_received_events),
            icon: item.icon.clone(),
        }
    }

    pub fn action(&self) -> &MenuItemAction {
        &self.action
    }

    pub fn icon(&self) -> Option<&String> {
        self.icon.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Menu { items }
    }

    pub fn from_config(
        menu: &config::types::Menu,
        app_received_events: Rc<RefCell<Vec<AppEvent>>>,
    ) -> Self {
        Menu {
            items: menu
                .items
                .iter()
                .map(|item| MenuItem::from_config(item, app_received_events.clone()))
                .collect(),
        }
    }
}

pub struct MenuSetup {
    pub menus: HashMap<MenuId, Menu>,
}

pub struct PieMenuInput {
    pub angle: f32,
    pub magnitude: f32,
    pub click: f32,
}

impl PieMenuInput {
    pub fn new(angle: f32, magnitude: f32, click: f32) -> Self {
        PieMenuInput {
            angle,
            magnitude,
            click,
        }
    }
}
