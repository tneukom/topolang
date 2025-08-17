use glow::HasContext;

use super::shader::Shader;
use crate::{
    camera::Camera,
    coordinate_frame::CoordinateFrames,
    math::{rect::Rect, rgba8::Rgba8},
    painting::{gl_buffer::GlVertexArrayObject, rect_vertices::RectVertices},
};

pub struct CheckerboardPainter {
    pub shader: Shader,
    pub vertices: RectVertices,
    pub vao: GlVertexArrayObject,
}

impl CheckerboardPainter {
    pub unsafe fn new(gl: &glow::Context) -> CheckerboardPainter {
        let vs_source = include_str!("shaders/checkerboard.vert");
        let fs_source = include_str!("shaders/checkerboard.frag");
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
        size: f64,
        even_rgba: Rgba8,
        odd_rgba: Rgba8,
        frames: &CoordinateFrames,
        camera: &Camera,
    ) {
        gl.disable(glow::BLEND);
        gl.disable(glow::FRAMEBUFFER_SRGB);

        self.vertices.update(gl, rect);
        self.vao.bind(gl);

        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "size", size);

        let device_from_world = frames.device_from_view() * camera.view_from_world();
        self.shader
            .uniform(gl, "device_from_world", &device_from_world);
        // Uniform assignment converts from SRGB to linear RGB
        self.shader
            .uniform(gl, "even_srgba", even_rgba.to_f32().to_array());
        self.shader
            .uniform(gl, "odd_srgba", odd_rgba.to_f32().to_array());

        // Draw 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
