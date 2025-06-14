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
#[serde(tag = "type", content = "key")]
pub enum KeyAction {
    Down(u16), // ScanCode
    Up(u16),   // ScanCode
}

pub type KeyStroke = Vec<KeyAction>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MenuItemAction {
    SubMenu {
        to: MenuId,
    },
    KeyStroke {
        key_stroke: KeyStroke,
    },
    Exec {
        program_path: String,
        args: Vec<String>,
    },
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
