use std::sync::Arc;

use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{point::Point, rect::Rect},
    painting::{
        line_painter::LinePainter, material_map_painter::MaterialMapPainter,
        rect_painter::RectPainter, selection_outline_painter::SelectionOutlinePainter,
    },
    pixmap::MaterialMap,
    view::{UiState, View},
};

use super::grid_painter::GridPainter;

pub struct ViewPainter {
    pub grid_painter: GridPainter,
    pub tile_painter: RectPainter,
    pub line_painter: LinePainter,
    pub world_painter: MaterialMapPainter,
    pub selection_painter: MaterialMapPainter,
    pub selection_outline_painter: SelectionOutlinePainter,

    pub i_frame: usize,
}

impl ViewPainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> ViewPainter {
        ViewPainter {
            grid_painter: GridPainter::new(gl.clone()),
            tile_painter: RectPainter::new(gl.clone()),
            line_painter: LinePainter::new(gl.clone()),
            world_painter: MaterialMapPainter::new(gl.clone(), 1024),
            selection_painter: MaterialMapPainter::new(gl.clone(), 1024),
            selection_outline_painter: SelectionOutlinePainter::new(gl.clone()),
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
        painter: &mut MaterialMapPainter,
        material_map: MaterialMap,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        painter.update(material_map);
        let world_to_device = frames.view_to_device() * camera.world_to_view();
        painter.draw(world_to_device, time);
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
            .draw_rect(bounds.cwise_cast(), world_to_device, time);
    }

    pub unsafe fn draw_selection_outline(
        &mut self,
        rect: Rect<i64>,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        let world_to_device = frames.view_to_device() * camera.world_to_view();
        self.selection_outline_painter
            .draw(&[rect.cwise_cast()], world_to_device, time);
    }

    pub unsafe fn draw_view(&mut self, view: &View, frames: &CoordinateFrames, time: f64) {
        // Grid in the background
        self.draw_grid(&view.camera, &frames);

        // actual scene
        Self::draw_material_map(
            &mut self.world_painter,
            view.world.material_map().clone(),
            &view.camera,
            &frames,
            time,
        );

        // Draw a rectangle around the scene
        // TODO:SPEEDUP: Use world.bounding_rect() instead, we don't want to call topology(),
        //   might be expensive.
        let bounding_rect = view.world.bounding_rect();
        self.draw_bounds(bounding_rect, &view.camera, &frames, time);

        // Draw selection rectangle and content
        if let Some(selection) = &view.selection {
            Self::draw_material_map(
                &mut self.selection_painter,
                selection.material_map().clone(),
                &view.camera,
                &frames,
                time,
            );

            self.draw_selection_outline(selection.bounding_rect(), &view.camera, frames, time);
        }

        // Draw selection rectangle currently being drawn
        if let UiState::SelectingRect(selecting) = &view.ui_state {
            self.draw_selection_outline(selecting.rect(), &view.camera, frames, time);
        }

        self.i_frame += 1;
    }
}
