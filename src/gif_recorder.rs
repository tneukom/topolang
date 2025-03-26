use crate::{
    field::RgbaField, material_effects::material_map_effects, math::rgba8::Rgba8,
    pixmap::MaterialMap,
};
use image::{
    codecs::gif::{GifEncoder, Repeat},
    ExtendedColorType,
};
use std::{fs::File, path::Path};

pub struct GifRecorder {
    pub frames: Vec<RgbaField>,
}

impl GifRecorder {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn add_frame(&mut self, material_map: &MaterialMap) {
        let mut rgba_field = RgbaField::filled(material_map.bounding_rect(), Rgba8::BLACK);
        material_map_effects(material_map, &mut rgba_field);
        self.frames.push(rgba_field);
    }

    pub fn export(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut gif_encoder = GifEncoder::new_with_speed(file, 10);
        gif_encoder.set_repeat(Repeat::Infinite).unwrap();

        for frame in &self.frames {
            gif_encoder.encode(
                frame.as_raw(),
                frame.width() as u32,
                frame.height() as u32,
                ExtendedColorType::Rgba8,
            )?;
        }

        Ok(())
    }
}
