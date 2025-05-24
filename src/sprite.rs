use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use tiny_skia::{IntRect, Pixmap};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sprite {
    name: String,
    width: u32,
    height: u32,
    x_start: i32,
    y_start: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpriteSheetMeta {
    sprites: HashMap<String, Sprite>,
    sheet: String,
}

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    meta: SpriteSheetMeta,
    pixmap: Pixmap,
}

impl SpriteSheet {
    pub fn load(path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let sprite_sheet_meta: SpriteSheetMeta =
            serde_json::from_reader(file).map_err(|e| e.to_string())?;
        let sheet_path: PathBuf = PathBuf::from(path.to_string() + "/" + &sprite_sheet_meta.sheet);

        let pixmap = Pixmap::load_png(sheet_path).map_err(|e| e.to_string())?;

        Ok(Self {
            meta: sprite_sheet_meta,
            pixmap,
        })
    }

    pub fn cutout(&self, name: &str) -> Option<Pixmap> {
        let Some(sprite) = self.meta.sprites.get(name) else {
            return None;
        };

        let Some(rect) =
            IntRect::from_xywh(sprite.x_start, sprite.y_start, sprite.width, sprite.height)
        else {
            return None;
        };

        self.pixmap.clone_rect(rect)
    }
}
