use std::sync::Arc;

use crate::{
    bitmap::Bitmap,
    material::Material,
    math::{affine_map::AffineMap, rect::Rect, rgba8::Rgba8},
    painting::{
        gl_texture::{Filter, GlTexture},
        rect_painter::{DrawRect, RectPainter},
    },
    pixmap::{PixmapMaterial, PixmapRgba},
};

pub struct ColorMapPainter {
    color_map: PixmapMaterial,
    rect_painter: RectPainter,
    texture: GlTexture,
}

impl ColorMapPainter {
    pub unsafe fn new(gl: Arc<glow::Context>, size: usize) -> Self {
        let mut texture = GlTexture::from_size(gl.clone(), size, size, Filter::Nearest);
        // TODO:SPEEDUP: Do we need to create an empty bitmap just to clear the texture?
        let bitmap = Bitmap::filled(size, size, Rgba8::TRANSPARENT);
        texture.texture_image(&bitmap);

        let color_map = PixmapMaterial::filled(Material::TRANSPARENT);

        Self {
            color_map,
            texture,
            rect_painter: RectPainter::new(gl),
        }
    }

    pub unsafe fn update(&mut self, material_map: PixmapMaterial) {
        for tile_index in PixmapRgba::tile_indices() {
            let current_tile = self.color_map.get_rc_tile(tile_index);
            let new_tile = material_map.get_rc_tile(tile_index);
            if PixmapMaterial::tile_ptr_eq(current_tile, new_tile) {
                continue;
            }

            if let Some(new_tile) = new_tile {
                // blit sub texture
                let color_field = new_tile
                    .fill_none(Material::TRANSPARENT)
                    .map_into::<Rgba8>();
                let tile_offset = PixmapRgba::tile_rect(tile_index).low();
                self.texture.texture_sub_image(tile_offset, &color_field);
            } else {
                // clear sub texture
                // TODO: Implement
            }
        }

        self.color_map = material_map;
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
