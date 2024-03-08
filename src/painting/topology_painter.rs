use crate::{
    bitmap::Bitmap,
    frozen::{counter, Frozen},
    math::{affine_map::AffineMap, generic::CwiseMul, rect::Rect},
    painting::{
        gl_texture::{Filter, GlTexture},
        tile_painter::{DrawRect, RectPainter},
        tile_store::TileStore,
    },
    topology::Topology,
};
use std::sync::Arc;

pub struct TopologyPainter {
    update_time: usize,
    rect_painter: RectPainter,
    tile_store: TileStore,
    texture: GlTexture,
}

impl TopologyPainter {
    pub unsafe fn new(gl: Arc<glow::Context>, size: usize) -> Self {
        let mut texture = GlTexture::from_size(gl.clone(), size, size, Filter::Nearest);
        let bitmap = Bitmap::transparent(size, size);
        texture.texture_image(&bitmap);
        Self {
            update_time: 0,
            texture,
            tile_store: TileStore::new(),
            rect_painter: RectPainter::new(gl),
        }
    }

    /// Has to be called with same instance of Topology every time
    pub unsafe fn update(&mut self, topology: &Topology) {
        self.tile_store.update(topology);

        // update all texture sub rectangles that have changed since `self.update_time`
        for (&tile_index, tile) in self.tile_store.iter_modified_after(self.update_time) {
            let Ok(usize_tile_index) = tile_index.cwise_try_into::<usize>() else {
                println!("negative tile indices not supported!");
                continue;
            };
            // println!("updating texture tile {}", Frozen::modified_time(tile));
            let offset = tile.size().cwise_mul(usize_tile_index);
            self.texture.texture_sub_image(offset, &tile);
        }

        self.update_time = counter();
    }

    /// Draw whole texture as a single rectangle
    pub unsafe fn draw(&mut self, to_glwindow: AffineMap<f64>) {
        let texture_rect = Rect::low_size([0, 0], self.texture.size());
        let draw_tile = DrawRect {
            texture_rect,
            corners: texture_rect.cwise_into_lossy().corners(),
        };

        self.rect_painter
            .draw(&[draw_tile], &self.texture, to_glwindow);
    }
}
