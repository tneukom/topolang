use crate::{
    field::RgbaField,
    material_effects::{material_map_effects, CHECKERBOARD_EVEN_RGBA, CHECKERBOARD_ODD_RGBA},
    math::rgba8::Rgba8,
    pixmap::MaterialMap,
};
use image::{
    codecs::gif::{GifEncoder, Repeat},
    Delay, Frame, RgbaImage,
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
        let rgba_field = material_map_effects(material_map, Rgba8::TRANSPARENT);
        self.frames.push(rgba_field);
    }

    pub fn export(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut gif_encoder = GifEncoder::new(file);
        gif_encoder.set_repeat(Repeat::Infinite)?;

        for mut frame in self.frames {
            // Alpha blend with checkerboard background
            for (pixel, color) in frame.enumerate_mut() {
                let checkerboard_sign = (pixel.x / 16 + pixel.y / 16) % 2 == 0;
                let checkerboard_rgba = if checkerboard_sign {
                    CHECKERBOARD_EVEN_RGBA
                } else {
                    CHECKERBOARD_ODD_RGBA
                };
                let composite_rgb = color.blend_rgb(checkerboard_rgba.rgb());
                *color = Rgba8::from_rgb_a(composite_rgb, 0xFF);
            }

            // Convert Field<Rgba8> to Vec<u8> then to image::RgbaImage
            let (width, height) = (frame.width(), frame.height());
            let raw_frame: Vec<u8> = bytemuck::cast_vec(frame.into_vec());
            let rgba_image = RgbaImage::from_raw(width as u32, height as u32, raw_frame).unwrap();

            // Add frame to gif
            let gif_frame = Frame::from_parts(rgba_image, 0, 0, Delay::from_numer_denom_ms(200, 1));
            gif_encoder.encode_frame(gif_frame)?;
        }

        Ok(())
    }
}
