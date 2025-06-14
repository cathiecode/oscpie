use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct MenuId(String);

impl MenuId {
    pub fn inner(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MenuItemAction {
    SubMenu { to: MenuId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub action: MenuItemAction,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub menus: HashMap<MenuId, Menu>,
    pub root: MenuId,
    pub sprite_sheet: String,
}
