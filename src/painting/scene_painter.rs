use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{point::Point, rect::Rect},
    painting::{line_painter::LinePainter, tile_painter::TilePainter},
    pixmap::Pixmap,
};
use std::sync::Arc;

use super::grid_painter::GridPainter;

pub struct ScenePainter {
    pub grid_painter: GridPainter,
    pub tile_painter: TilePainter,
    pub line_painter: LinePainter,

    pub i_frame: usize,
}

impl ScenePainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> ScenePainter {
        ScenePainter {
            grid_painter: GridPainter::new(gl.clone()),
            tile_painter: TilePainter::new(gl.clone()),
            line_painter: LinePainter::new(gl.clone()),
            i_frame: 0,
        }
    }

    pub unsafe fn draw_grid(&self, camera: &Camera, frames: &CoordinateFrames) {
        let world_to_pixelwindow = frames.view_to_pixelwindow() * camera.world_to_view();
        let origin = world_to_pixelwindow * Point::ZERO;
        let spacing = world_to_pixelwindow.linear * Point::ONE;
        self.grid_painter.draw(origin, spacing, frames);
    }

    pub unsafe fn draw_pixmap(
        &mut self,
        pixmap: &Pixmap,
        camera: &Camera,
        frames: &CoordinateFrames,
    ) {
        let (atlas, draw_tiles) = TilePainter::pixmap_draw_tiles(pixmap);

        let world_to_glwindow = frames.view_to_glwindow() * camera.world_to_view();
        self.tile_painter
            .draw(&draw_tiles, &atlas, world_to_glwindow);
    }

    pub unsafe fn draw_bounds(
        &mut self,
        bounds: Rect<i64>,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        let world_to_glwindow = frames.view_to_glwindow() * camera.world_to_view();
        self.line_painter
            .draw_rect(bounds.cwise_into_lossy(), world_to_glwindow, time);
    }
}
