use std::mem::{offset_of, size_of};

use crate::{
    math::rect::Rect,
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget},
        shader::{Shader, VertexAttribDesc},
    },
};

#[derive(Debug, Clone, Copy)]
struct Vertex {
    pub position: [f32; 2],
}

pub struct RectVertices {
    array_buffer: GlBuffer<Vertex>,
}

impl RectVertices {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        Self { array_buffer }
    }

    pub unsafe fn assign_attribute(&self, gl: &glow::Context, shader: &Shader, name: &str) {
        let size = size_of::<Vertex>();
        self.array_buffer.bind(gl);
        shader.assign_attribute_f32(
            gl,
            name,
            &VertexAttribDesc::VEC2,
            offset_of!(Vertex, position) as i32,
            size as i32,
        );
    }

    pub unsafe fn update(&self, gl: &glow::Context, rect: Rect<f64>) {
        // Make sure we can render a triangle strip
        let corners = [
            rect.bottom_left(),
            rect.bottom_right(),
            rect.top_left(),
            rect.top_right(),
        ];
        let vertices = corners.map(|corner| Vertex {
            position: corner.as_f32().to_array(),
        });

        self.array_buffer.bind(gl);
        self.array_buffer.buffer_data(gl, &vertices);
    }
}

pub const RECT_TRIANGLE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
