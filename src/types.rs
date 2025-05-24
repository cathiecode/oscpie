use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuId(String);

impl MenuId {
    pub fn new(id: String) -> Self {
        MenuId(id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemAction {
    Noop,
    SubMenu { to: MenuId },
}

pub struct MenuItem {
    pub action: MenuItemAction,
}

pub struct Menu {
    pub items: Vec<MenuItem>,
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
