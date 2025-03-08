use glow::HasContext;

use super::shader::Shader;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{matrix3::Matrix3, rect::Rect},
    painting::rect_vertices::RectVertices,
};

pub struct CheckerboardPainter {
    pub shader: Shader,
    pub vertices: RectVertices,
}

impl CheckerboardPainter {
    pub unsafe fn new(gl: &glow::Context) -> CheckerboardPainter {
        let vs_source = include_str!("shaders/checkerboard.vert");
        let fs_source = include_str!("shaders/checkerboard.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        let vertices = RectVertices::new(gl);
        vertices.assign_attribute(gl, &shader, "in_world_position");

        Self { shader, vertices }
    }

    // Draw a grid, the lines are 1 pixel wide.
    // The offset and spacing are in the view coordinate system
    pub unsafe fn draw(
        &self,
        gl: &glow::Context,
        rect: Rect<f64>,
        size: f64,
        frames: &CoordinateFrames,
        camera: &Camera,
    ) {
        gl.disable(glow::BLEND);

        self.vertices.bind_vertices(gl, rect);

        let world_to_device = frames.view_to_device() * camera.world_to_view();
        let mat_world_to_device = Matrix3::from(world_to_device);

        self.shader.use_program(gl);
        self.shader.uniform(gl, "size", size);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        self.vertices.draw_elements(gl);
    }
}
