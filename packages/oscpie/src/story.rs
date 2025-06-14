use std::path::PathBuf;

use tiny_skia::Pixmap;

fn pixmap() -> Pixmap {
    let mut pixmap = Pixmap::new(512, 512).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 255));
    pixmap
}

fn save_pixmap(pixmap: &Pixmap, filename: &str) {
    pixmap
        .save_png(PathBuf::from("stories/".to_string() + filename))
        .unwrap();
}

pub fn story<F>(name: &str, f: F)
where
    F: FnOnce(&mut Pixmap),
{
    let mut pixmap = pixmap();
    f(&mut pixmap);
    save_pixmap(&pixmap, format!("{name}.png").as_str());
}
