use std::sync::Arc;

use crate::{
    bitmap::Bitmap,
    math::{affine_map::AffineMap, rect::Rect, rgba8::Rgba8},
    painting::{
        gl_texture::{Filter, GlTexture},
        rect_painter::{DrawRect, RectPainter},
    },
    pixmap::PixmapRgba,
};

pub struct ColorMapPainter {
    color_map: PixmapRgba,
    rect_painter: RectPainter,
    texture: GlTexture,
}

impl ColorMapPainter {
    pub unsafe fn new(gl: Arc<glow::Context>, size: usize) -> Self {
        let mut texture = GlTexture::from_size(gl.clone(), size, size, Filter::Nearest);
        // TODO:SPEEDUP: Do we need to create an empty bitmap just to clear the texture?
        let bitmap = Bitmap::filled(size, size, Rgba8::TRANSPARENT);
        texture.texture_image(&bitmap);

        let color_map = PixmapRgba::filled(Rgba8::TRANSPARENT);

        Self {
            color_map,
            texture,
            rect_painter: RectPainter::new(gl),
        }
    }

    pub unsafe fn update(&mut self, color_map: PixmapRgba) {
        for tile_index in PixmapRgba::tile_indices() {
            let current_tile = self.color_map.get_rc_tile(tile_index);
            let new_tile = color_map.get_rc_tile(tile_index);
            if PixmapRgba::tile_ptr_eq(current_tile, new_tile) {
                continue;
            }

            if let Some(new_tile) = new_tile {
                // blit sub texture
                let color_field = new_tile.fill_none(Rgba8::TRANSPARENT);
                let tile_offset = PixmapRgba::tile_rect(tile_index).low();
                self.texture.texture_sub_image(tile_offset, &color_field);
            } else {
                // clear sub texture
                // TODO: Implement
            }
        }

        self.color_map = color_map;
    }

    /// Draw whole texture as a single rectangle
    pub unsafe fn draw(&mut self, to_device: AffineMap<f64>, time: f64) {
        let texture_rect = Rect::low_size([0, 0], self.texture.size());
        let draw_tile = DrawRect {
            texture_rect,
            corners: texture_rect.cwise_into_lossy().corners(),
        };

        self.rect_painter
            .draw(&[draw_tile], &self.texture, to_device, time);
    }
}
