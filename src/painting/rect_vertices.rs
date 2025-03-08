use std::mem::{offset_of, size_of};

use glow::HasContext;

use crate::{
    math::rect::Rect,
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
        shader::{Shader, VertexAttribDesc},
    },
};

#[derive(Debug, Clone, Copy)]
struct Vertex {
    pub position: [f32; 2],
}

pub struct RectVertices {
    array_buffer: GlBuffer<Vertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
}

impl RectVertices {
    const INDICES_LEN: usize = 6;

    pub unsafe fn new(gl: &glow::Context) -> Self {
        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);
        vertex_array.unbind(gl);

        let indices = Rect::<f64>::TRIANGLE_INDICES.map(|i| i as u32);
        assert_eq!(indices.len(), Self::INDICES_LEN);
        element_buffer.buffer_data(gl, &indices);

        Self {
            array_buffer,
            element_buffer,
            vertex_array,
        }
    }

    pub unsafe fn assign_attribute(&self, gl: &glow::Context, shader: &Shader, name: &str) {
        self.vertex_array.bind(gl);
        let size = size_of::<Vertex>();
        shader.assign_attribute_f32(
            gl,
            name,
            &VertexAttribDesc::VEC2,
            offset_of!(Vertex, position) as i32,
            size as i32,
        );
    }

    pub unsafe fn bind_vertices(&self, gl: &glow::Context, rect: Rect<f64>) {
        let vertices = rect.corners().map(|corner| Vertex {
            position: corner.as_f32().to_array(),
        });

        self.vertex_array.bind(gl);
        self.array_buffer.buffer_data(gl, &vertices);
    }

    pub unsafe fn draw_elements(&self, gl: &glow::Context) {
        gl.draw_elements(
            glow::TRIANGLES,
            Self::INDICES_LEN as i32,
            glow::UNSIGNED_INT,
            0,
        );
    }
}
