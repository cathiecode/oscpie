use std::sync::OnceLock;

use crate::sprite::SpriteSheet;

pub static SPRITE_SHEET: OnceLock<SpriteSheet> = OnceLock::new();

pub fn get_sprite_sheet() -> Option<&'static SpriteSheet> {
    SPRITE_SHEET.get()
}
