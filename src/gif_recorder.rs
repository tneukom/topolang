use crate::{history::Snapshot, material::Material};
use anyhow::Context;
use image::{
    codecs::gif::{GifEncoder, Repeat},
    ExtendedColorType,
};
use std::{fs::File, path::Path, rc::Rc};

pub struct GifRecorder {
    pub start: Option<Rc<Snapshot>>,
    pub stop: Option<Rc<Snapshot>>,
}

impl GifRecorder {
    pub fn new() -> Self {
        Self {
            start: None,
            stop: None,
        }
    }

    /// Try to find a path from start to stop or from stop to start
    fn history_path(&self) -> Option<Vec<&Rc<Snapshot>>> {
        let start = self.start.as_ref()?;
        let stop = self.stop.as_ref()?;
        if let Some(path) = stop.path_to(start) {
            return Some(path);
        }
        if let Some(path) = start.path_to(stop) {
            return Some(path);
        }
        None
    }

    pub fn export(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let snapshot_path = self.history_path().context("No path")?;

        let file = File::create(path)?;
        let mut gif_encoder = GifEncoder::new_with_speed(file, 10);
        gif_encoder.set_repeat(Repeat::Infinite).unwrap();

        for snapshot in snapshot_path {
            // TODO: Offset world pixmap by bounds.low()
            // TODO: Paint over white background to remove transparency or use apng instead
            let image = snapshot
                .material_map()
                .to_rgba8_field(Material::TRANSPARENT);

            gif_encoder.encode(
                image.as_raw(),
                image.width() as u32,
                image.height() as u32,
                ExtendedColorType::Rgba8,
            )?;
        }

        Ok(())
    }
}
