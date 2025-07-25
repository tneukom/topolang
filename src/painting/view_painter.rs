use super::grid_painter::GridPainter;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    field::RgbaField,
    material::Material,
    material_effects::{CHECKERBOARD_EVEN_RGBA, CHECKERBOARD_ODD_RGBA},
    math::rect::Rect,
    painting::{
        checkerboard_painter::CheckerboardPainter, line_painter::LinePainter,
        material_map_painter::RgbaFieldPainter,
    },
    view::{DraggingKind, UiState, View, ViewInput, ViewSettings},
};
use std::sync::{Arc, RwLock};

/// What is necessary to paint the view
pub struct DrawView {
    camera: Camera,
    frames: CoordinateFrames,
    time: f64,
    world_rgba_field: Arc<RwLock<RgbaField>>,
    world_rgba_expired: Rect<i64>,
    selection_rgba_field: Option<Arc<RgbaField>>,
    overlay_rgba_field: Option<RgbaField>,
    ui_state: UiState,
    grid_size: Option<i64>,
}

impl DrawView {
    pub fn from_view(
        view: &View,
        view_settings: &ViewSettings,
        view_input: &ViewInput,
        frames: CoordinateFrames,
        time: f64,
    ) -> Self {
        let (world_rgba_field, world_rgba_expired) = view.world.fresh_rgba_field();

        let selection_rgba_field = view
            .selection
            .as_ref()
            .map(|selection| selection.rgba_field().clone());

        // TODO: Currently without effects
        let overlay_rgba_field = view
            .overlay(view_settings, view_input)
            .map(|material_map| material_map.to_rgba_field(Material::TRANSPARENT));

        Self {
            ui_state: view.ui_state.clone(),
            camera: view.camera,
            grid_size: view.grid_size,
            world_rgba_field,
            selection_rgba_field,
            overlay_rgba_field,
            world_rgba_expired,
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
    pub overlay_painter: RgbaFieldPainter,
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
            overlay_painter: RgbaFieldPainter::new(gl),
            selection_painter: RgbaFieldPainter::new(gl),
            i_frame: 0,
        }
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
        // let world_to_view = draw.frames.
        let world_to_device = draw.frames.view_to_device() * draw.camera.world_to_view();
        let read_world_rgba_field = draw.world_rgba_field.read().unwrap();

        // Checkerboard pattern in background
        let world_bounds = read_world_rgba_field.bounds().cwise_as();
        self.checkerboard_painter.draw(
            gl,
            world_bounds,
            16.0,
            CHECKERBOARD_EVEN_RGBA,
            CHECKERBOARD_ODD_RGBA,
            &draw.frames,
            &draw.camera,
        );

        self.world_painter.draw(
            gl,
            &read_world_rgba_field,
            draw.world_rgba_expired,
            draw.camera.world_to_view(),
            draw.frames.view_to_device(),
            draw.time,
        );

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
                self.selection_painter.draw(
                    gl,
                    &selection_rgba_field,
                    selection_rgba_field.bounds(),
                    draw.camera.world_to_view(),
                    draw.frames.view_to_device(),
                    draw.time,
                );

                self.draw_selection_outline(
                    gl,
                    selection_rgba_field.bounds(),
                    &draw.camera,
                    &draw.frames,
                    draw.time,
                );
            }
        }

        if let Some(overlay_rgba_field) = &draw.overlay_rgba_field {
            self.overlay_painter.draw(
                gl,
                &overlay_rgba_field,
                overlay_rgba_field.bounds(),
                draw.camera.world_to_view(),
                draw.frames.view_to_device(),
                draw.time,
            );
        }

        // Draw selection rectangle currently being drawn
        if let UiState::Dragging(selecting) = &draw.ui_state {
            if selecting.kind == DraggingKind::SelectRect {
                self.draw_selection_outline(
                    gl,
                    selecting.rect(),
                    &draw.camera,
                    &draw.frames,
                    draw.time,
                );
            }
        }

        // Grid in the background
        if let Some(grid_size) = draw.grid_size {
            self.grid_painter.draw(
                gl,
                world_bounds,
                grid_size as f64,
                &draw.frames,
                &draw.camera,
            );
        }

        self.i_frame += 1;
    }
}
