use glow::HasContext;

use super::shader::Shader;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{matrix3::Matrix3, rect::Rect},
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

        let world_to_view = camera.world_to_view();
        let mat_world_to_view = Matrix3::from(world_to_view);
        self.shader.uniform(gl, "world_to_view", &mat_world_to_view);

        let world_to_device = frames.view_to_device() * camera.world_to_view();
        let mat_world_to_device = Matrix3::from(world_to_device);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
