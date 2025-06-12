use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct MenuId(String);

impl From<MenuId> for crate::types::MenuId {
    fn from(val: MenuId) -> Self {
        crate::types::MenuId::new(val.0.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MenuItemAction {
    SubMenu { to: MenuId },
}

impl From<MenuItemAction> for crate::types::MenuItemAction {
    fn from(val: MenuItemAction) -> Self {
        match val {
            MenuItemAction::SubMenu { to } => {
                crate::types::MenuItemAction::PushStack { to: to.into() }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub action: MenuItemAction,
    pub icon: Option<String>,
}

impl From<MenuItem> for crate::types::MenuItem {
    fn from(val: MenuItem) -> Self {
        crate::types::MenuItem {
            action: val.action.into(),
            icon: val.icon,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

impl From<Menu> for crate::types::Menu {
    fn from(val: Menu) -> Self {
        crate::types::Menu {
            items: val
                .items
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub menus: HashMap<MenuId, Menu>,
    pub root: MenuId,
    pub sprite_sheet: String,
}
