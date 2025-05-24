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
    image: String,
}

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    meta: SpriteSheetMeta,
    pixmap: Pixmap,
}

impl SpriteSheet {
    pub fn load(path: &str) -> Result<Self, String> {
        let sheet_path = PathBuf::from(path);
        let file = std::fs::File::open(&sheet_path).map_err(|e| e.to_string())?;
        let sprite_sheet_meta: SpriteSheetMeta =
            serde_json::from_reader(file).map_err(|e| e.to_string())?;
        
        let image_path: PathBuf = sheet_path.parent().unwrap().join(sprite_sheet_meta.image.clone());

        log::info!("Loading sprite sheet: {}", image_path.display());

        let pixmap = Pixmap::load_png(image_path.clone()).map_err(|e| format!("{}: {}", e.to_string(), image_path.display()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_sprite_sheet() -> SpriteSheet {
        SpriteSheet::load("test_files/sprites/sprites.json").unwrap()
    }

    #[test]
    fn test_load_sprite_sheet() {
        SpriteSheet::load("test_files/sprites/sprites.json").unwrap();
    }

    #[test]
    fn test_cutout_sprite() {
        let sprite_sheet = load_test_sprite_sheet();

        let sprite_s = sprite_sheet.cutout("s");
        assert!(sprite_s.is_some());

        assert_eq!(Pixmap::load_png("test_files/sprites/s.png").unwrap(), sprite_s.unwrap());

        let sprite_p = sprite_sheet.cutout("p");
        assert!(sprite_p.is_some());

        assert_ne!(Pixmap::load_png("test_files/sprites/s.png").unwrap(), sprite_p.unwrap());
    }
}
