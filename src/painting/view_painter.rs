use std::sync::Arc;

use super::grid_painter::GridPainter;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    material::Material,
    math::{point::Point, rect::Rect},
    painting::{
        line_painter::LinePainter, material_map_painter::RgbaFieldPainter,
        rect_painter::RectPainter, selection_outline_painter::SelectionOutlinePainter,
    },
    view::{UiState, View},
};

/// What is necessary to paint the view
pub struct DrawView {
    camera: Camera,
    frames: CoordinateFrames,
    time: f64,
    world_rgba_field: RgbaField,
    // TODO: Can we use world_rgba_field for this?
    selection_rgba_field: Option<RgbaField>,
    ui_state: UiState,
}

impl DrawView {
    pub fn from_view(view: &View, frames: CoordinateFrames, time: f64) -> Self {
        let world_rgba_field = view
            .world
            .material_map()
            .to_rgba8_field(Material::TRANSPARENT);
        let selection_rgba_field = view.selection.as_ref().map(|selection| {
            selection
                .material_map()
                .to_rgba8_field(Material::TRANSPARENT)
        });

        Self {
            ui_state: view.ui_state.clone(),
            camera: view.camera,
            world_rgba_field,
            selection_rgba_field,
            frames,
            time,
        }
    }
}

pub struct ViewPainter {
    pub grid_painter: GridPainter,
    pub tile_painter: RectPainter,
    pub line_painter: LinePainter,
    pub world_painter: RgbaFieldPainter,
    pub selection_painter: RgbaFieldPainter,
    pub selection_outline_painter: SelectionOutlinePainter,

    pub i_frame: usize,
}

impl ViewPainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> ViewPainter {
        ViewPainter {
            grid_painter: GridPainter::new(gl.clone()),
            tile_painter: RectPainter::new(gl.clone()),
            line_painter: LinePainter::new(gl.clone()),
            world_painter: RgbaFieldPainter::new(gl.clone()),
            selection_painter: RgbaFieldPainter::new(gl.clone()),
            selection_outline_painter: SelectionOutlinePainter::new(gl.clone()),
            i_frame: 0,
        }
    }

    pub unsafe fn draw_grid(&self, camera: &Camera, frames: &CoordinateFrames) {
        let origin = camera.world_to_view() * Point::ZERO;
        let spacing = camera.world_to_view().linear * Point::ONE;
        self.grid_painter.draw(origin, spacing, frames);
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
            .draw_rect(bounds.cwise_as(), world_to_device, time);
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
            .draw(&[rect.cwise_as()], world_to_device, time);
    }

    pub unsafe fn draw_view(&mut self, draw: &DrawView) {
        let world_to_device = draw.frames.view_to_device() * draw.camera.world_to_view();

        // Grid in the background
        self.draw_grid(&draw.camera, &draw.frames);

        self.world_painter
            .draw(&draw.world_rgba_field, world_to_device, draw.time);

        // Draw a rectangle around the scene
        self.draw_bounds(
            draw.world_rgba_field.bounds(),
            &draw.camera,
            &draw.frames,
            draw.time,
        );

        // Draw selection rectangle and content
        if let Some(selection_rgba_field) = &draw.selection_rgba_field {
            if !selection_rgba_field.is_empty() {
                self.selection_painter
                    .draw(&selection_rgba_field, world_to_device, draw.time);

                self.draw_selection_outline(
                    selection_rgba_field.bounds(),
                    &draw.camera,
                    &draw.frames,
                    draw.time,
                );
            }
        }

        // Draw selection rectangle currently being drawn
        if let UiState::SelectingRect(selecting) = &draw.ui_state {
            self.draw_selection_outline(selecting.rect(), &draw.camera, &draw.frames, draw.time);
        }

        self.i_frame += 1;
    }
}
