use std::sync::Arc;

use crate::{
    field::{Field, RgbaField},
    material::Material,
    math::{affine_map::AffineMap, point::Point, rect::Rect, rgba8::Rgba8},
    painting::{
        gl_texture::{Filter, GlTexture},
        rect_painter::{DrawRect, RectPainter},
    },
    pixmap,
    pixmap::{MaterialMap, Tile},
};

enum Diff {
    Modified,
    Added,
    Removed,
}

pub struct MaterialMapPainter {
    clear_tile: Field<Rgba8>,
    color_map: MaterialMap,
    rect_painter: RectPainter,
    texture: GlTexture,
}

impl MaterialMapPainter {
    pub unsafe fn new(gl: Arc<glow::Context>, size: i64) -> Self {
        let mut texture = GlTexture::from_size(gl.clone(), size, size, Filter::Nearest);
        // TODO:SPEEDUP: Do we need to create an empty bitmap just to clear the texture?
        let bitmap = RgbaField::filled(Rect::low_size([0, 0], [size, size]), Rgba8::TRANSPARENT);
        texture.texture_image(&bitmap);

        let color_map = MaterialMap::new();
        let clear_tile = Tile::new().fill_none(Material::TRANSPARENT).into_rgba();

        Self {
            clear_tile,
            color_map,
            texture,
            rect_painter: RectPainter::new(gl),
        }
    }

    fn diffs(before: &MaterialMap, after: &MaterialMap) -> Vec<(Point<i64>, Diff)> {
        let mut diffs = Vec::new();

        for (&tile_index, before_tile) in &before.tiles {
            let after_tile = after.tiles.get(&tile_index);
            if let Some(after_tile) = after_tile {
                if before_tile != after_tile {
                    diffs.push((tile_index, Diff::Modified));
                }
            } else {
                diffs.push((tile_index, Diff::Removed));
            }
        }

        for &tile_index in after.tiles.keys() {
            let before_tile = before.tiles.get(&tile_index);
            if before_tile.is_none() {
                diffs.push((tile_index, Diff::Added));
            }
        }

        diffs
    }

    pub unsafe fn update(&mut self, material_map: MaterialMap) {
        let diffs = Self::diffs(&self.color_map, &material_map);

        for (tile_index, _) in diffs {
            let after_tile = material_map.tiles.get(&tile_index);

            // let color_field = if let Some(new_tile) = new_tile {
            //     new_tile
            //         .fill_none(Material::TRANSPARENT)
            //         .map_into::<Rgba8>()
            // } else {
            //     // TODO: Directly use Field<Rgba8> of appropriate size, cache
            //     Tile::new()
            //         .fill_none(Material::TRANSPARENT)
            //         .map_into::<Rgba8>()
            // };

            let color_field = match after_tile {
                // TODO: Store empty Field<Rgba8>
                None => &self.clear_tile,
                Some(after_tile) => &after_tile
                    .fill_none(Material::TRANSPARENT)
                    .map_into::<Rgba8>(),
            };

            // blit sub texture
            let tile_offset = pixmap::tile_rect(tile_index).low();
            self.texture.texture_sub_image(tile_offset, color_field);
        }

        // for tile_index in RgbaMap::tile_indices() {
        //     let current_tile = self.color_map.get_rc_tile(tile_index);
        //     let new_tile = material_map.get_rc_tile(tile_index);
        //     if MaterialMap::tile_ptr_eq(current_tile, new_tile) {
        //         continue;
        //     }
        //
        //     let tile_offset = RgbaMap::tile_rect(tile_index).low();
        //
        //
        //     // blit sub texture
        //     self.texture.texture_sub_image(tile_offset, &color_field);
        // }

        self.color_map = material_map;
    }

    /// Draw whole texture as a single rectangle
    pub unsafe fn draw(&mut self, to_device: AffineMap<f64>, time: f64) {
        let texture_rect = Rect::low_size([0, 0], self.texture.size());
        let draw_tile = DrawRect {
            texture_rect,
            corners: texture_rect.cwise_cast().corners(),
        };

        self.rect_painter
            .draw(&[draw_tile], &self.texture, to_device, time);
    }
}
