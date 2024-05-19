use std::sync::Arc;

use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{point::Point, rect::Rect},
    painting::{
        line_painter::LinePainter, rect_painter::RectPainter, topology_painter::ColorMapPainter,
    },
    pixmap::MaterialMap,
};

use super::grid_painter::GridPainter;

pub struct ScenePainter {
    pub grid_painter: GridPainter,
    pub tile_painter: RectPainter,
    pub line_painter: LinePainter,
    pub material_map_painter: ColorMapPainter,

    pub i_frame: usize,
}

impl ScenePainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> ScenePainter {
        ScenePainter {
            grid_painter: GridPainter::new(gl.clone()),
            tile_painter: RectPainter::new(gl.clone()),
            line_painter: LinePainter::new(gl.clone()),
            material_map_painter: ColorMapPainter::new(gl, 1024),
            i_frame: 0,
        }
    }

    pub unsafe fn draw_grid(&self, camera: &Camera, frames: &CoordinateFrames) {
        let world_to_window = frames.view_to_window() * camera.world_to_view();
        let origin = world_to_window * Point::ZERO;
        let spacing = world_to_window.linear * Point::ONE;
        self.grid_painter.draw(origin, spacing, frames);
    }

    pub unsafe fn draw_material_map(
        &mut self,
        material_map: MaterialMap,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        self.material_map_painter.update(material_map);

        let world_to_device = frames.view_to_device() * camera.world_to_view();
        self.material_map_painter.draw(world_to_device, time);
    }

    pub unsafe fn draw_bounds(
        &mut self,
        bounds: Rect<i64>,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        let world_to_device = frames.view_to_device() * camera.world_to_view();
        self.line_painter
            .draw_rect(bounds.cwise_into_lossy(), world_to_device, time);
    }
}
