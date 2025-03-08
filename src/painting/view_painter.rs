use super::grid_painter::GridPainter;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    material::Material,
    math::{point::Point, rect::Rect},
    painting::{
        checkerboard_painter::CheckerboardPainter, line_painter::LinePainter,
        material_map_painter::RgbaFieldPainter,
    },
    view::{EditMode, SelectingKind, UiState, View, ViewInput, ViewSettings},
};
use std::sync::{Arc, RwLock};

/// What is necessary to paint the view
pub struct DrawView {
    camera: Camera,
    frames: CoordinateFrames,
    time: f64,
    world_rgba_field: Arc<RwLock<RgbaField>>,
    // TODO: Can we use world_rgba_field for this?
    selection_rgba_field: Option<RgbaField>,
    brush_preview: Option<RgbaField>,
    ui_state: UiState,
}

impl DrawView {
    pub fn from_view(
        view: &View,
        view_settings: &ViewSettings,
        view_input: &ViewInput,
        frames: CoordinateFrames,
        time: f64,
    ) -> Self {
        let world_rgba_field = view.world.fresh_rgba_field();

        let selection_rgba_field = view.selection.as_ref().map(|selection| {
            selection
                .material_map()
                .to_rgba_field(Material::TRANSPARENT)
        });

        let brush_preview = if view_settings.edit_mode == EditMode::Brush {
            let world_mouse = view.camera.view_to_world() * view_input.view_mouse;
            Some(view_settings.brush.dot(world_mouse).into_rgba())
        } else {
            None
        };

        Self {
            ui_state: view.ui_state.clone(),
            camera: view.camera,
            world_rgba_field,
            selection_rgba_field,
            brush_preview,
            frames,
            time,
        }
    }
}

pub struct ViewPainter {
    pub checkerboard_painter: CheckerboardPainter,
    pub grid_painter: GridPainter,
    pub line_painter: LinePainter,
    pub world_painter: RgbaFieldPainter,
    pub brush_preview_painter: RgbaFieldPainter,
    pub selection_painter: RgbaFieldPainter,

    pub i_frame: usize,
}

impl ViewPainter {
    pub unsafe fn new(gl: &glow::Context) -> ViewPainter {
        ViewPainter {
            checkerboard_painter: CheckerboardPainter::new(gl),
            grid_painter: GridPainter::new(gl),
            line_painter: LinePainter::new(gl),
            world_painter: RgbaFieldPainter::new(gl),
            brush_preview_painter: RgbaFieldPainter::new(gl),
            selection_painter: RgbaFieldPainter::new(gl),
            i_frame: 0,
        }
    }

    pub unsafe fn draw_grid(&self, gl: &glow::Context, camera: &Camera, frames: &CoordinateFrames) {
        let origin = camera.world_to_view() * Point::ZERO;
        let spacing = camera.world_to_view().linear * Point::ONE;
        self.grid_painter.draw(gl, origin, spacing, frames);
    }

    pub unsafe fn draw_selection_outline(
        &mut self,
        gl: &glow::Context,
        rect: Rect<i64>,
        camera: &Camera,
        frames: &CoordinateFrames,
        time: f64,
    ) {
        let world_to_device = frames.view_to_device() * camera.world_to_view();
        self.line_painter
            .draw_rect(gl, rect.cwise_as(), world_to_device, time);
    }

    pub unsafe fn draw_view(&mut self, gl: &glow::Context, draw: &DrawView) {
        let world_to_device = draw.frames.view_to_device() * draw.camera.world_to_view();
        let read_world_rgba_field = draw.world_rgba_field.read().unwrap();

        // Checkerboard pattern in background
        let world_bounds = read_world_rgba_field.bounds().cwise_as();
        self.checkerboard_painter
            .draw(gl, world_bounds, 16.0, &draw.frames, &draw.camera);

        self.world_painter
            .draw(gl, &read_world_rgba_field, world_to_device, draw.time);

        // Draw a rectangle around the scene
        self.draw_selection_outline(
            gl,
            read_world_rgba_field.bounds(),
            &draw.camera,
            &draw.frames,
            draw.time,
        );

        // Draw selection rectangle and content
        if let Some(selection_rgba_field) = &draw.selection_rgba_field {
            if !selection_rgba_field.is_empty() {
                self.selection_painter
                    .draw(gl, &selection_rgba_field, world_to_device, draw.time);

                self.draw_selection_outline(
                    gl,
                    selection_rgba_field.bounds(),
                    &draw.camera,
                    &draw.frames,
                    draw.time,
                );
            }
        }

        // Draw selection rectangle currently being drawn
        if let UiState::SelectingRect(selecting) = &draw.ui_state {
            if selecting.kind == SelectingKind::Rect {
                self.draw_selection_outline(
                    gl,
                    selecting.rect(),
                    &draw.camera,
                    &draw.frames,
                    draw.time,
                );
            }
        }

        // Draw brush preview
        if let Some(brush_preview) = &draw.brush_preview {
            self.brush_preview_painter
                .draw(gl, brush_preview, world_to_device, draw.time);
        }

        // Grid in the background
        self.draw_grid(gl, &draw.camera, &draw.frames);

        self.i_frame += 1;
    }
}
