use std::sync::Arc;

use crate::{
    bitmap::Bitmap,
    frozen::counter,
    math::{affine_map::AffineMap, point::Point, rect::Rect},
    painting::{
        gl_texture::{Filter, GlTexture},
        rect_painter::{DrawRect, RectPainter},
    },
    topology::Topology,
};

pub struct TopologyPainter {
    update_time: usize,
    rect_painter: RectPainter,
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
            rect_painter: RectPainter::new(gl),
        }
    }

    /// Has to be called with same instance of Topology every time
    /// Currently updates the whole texture
    /// TODO: Track topology changes somehow, options
    ///   - Track if there were any changes at all since last update, using an mtime
    ///   - Track modified rectangle
    ///   - Track modified time on tiles
    pub unsafe fn update(&mut self, topology: &Topology) {
        let color_field = topology.color_field();
        self.texture.texture_sub_image(Point(0, 0), &color_field);
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
