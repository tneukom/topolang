use glow::HasContext;

use super::shader::Shader;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::rect::Rect,
    painting::{gl_buffer::GlVertexArrayObject, rect_vertices::RectVertices},
};

pub struct GridPainter {
    pub shader: Shader,
    pub vertices: RectVertices,
    pub vao: GlVertexArrayObject,
}

impl GridPainter {
    pub unsafe fn new(gl: &glow::Context) -> GridPainter {
        let vs_source = include_str!("shaders/grid.vert");
        let fs_source = include_str!("shaders/grid.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        let vertices = RectVertices::new(gl);
        let vao = GlVertexArrayObject::new(gl);
        vao.bind(gl);
        vertices.assign_attribute(gl, &shader, "in_world_position");

        Self {
            shader,
            vertices,
            vao,
        }
    }

    // Draw a grid, the lines are 1 pixel wide.
    // The offset and spacing are in the view coordinate system
    pub unsafe fn draw(
        &self,
        gl: &glow::Context,
        rect: Rect<f64>,
        spacing: f64,
        frames: &CoordinateFrames,
        camera: &Camera,
    ) {
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        self.vertices.update(gl, rect);

        self.vao.bind(gl);

        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "world_spacing", spacing);

        let view_from_world = camera.view_from_world();
        self.shader.uniform(gl, "world_to_view", &view_from_world);

        let device_from_world = frames.device_from_view() * camera.view_from_world();
        self.shader
            .uniform(gl, "device_from_world", &device_from_world);

        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
