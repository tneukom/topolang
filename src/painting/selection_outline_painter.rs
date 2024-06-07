use std::{
    mem::{offset_of, size_of},
    sync::Arc,
};

use glow::HasContext;

use crate::{
    math::{affine_map::AffineMap, rect::Rect},
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
        shader::{Shader, VertexAttribDesc},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct SelectionVertex {
    pub position: [f32; 2],
}

pub struct SelectionOutlinePainter {
    shader: Shader,
    array_buffer: GlBuffer<SelectionVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
    gl: Arc<glow::Context>,
}

impl SelectionOutlinePainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        let vs_source = include_str!("shaders/selection.vert");
        let fs_source = include_str!("shaders/selection.frag");
        let shader = Shader::from_source(gl.clone(), &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl.clone());

        vertex_array.bind();
        array_buffer.bind();
        element_buffer.bind();

        let size = size_of::<SelectionVertex>();
        shader.assign_attribute_f32(
            "in_glwindow_position",
            &VertexAttribDesc::VEC2,
            offset_of!(SelectionVertex, position) as i32,
            size as i32,
        );

        Self {
            shader,
            array_buffer,
            element_buffer,
            vertex_array,
            gl,
        }
    }

    /// Draw a list of selection rects, each rectangle is drawn as 4 GL_LINES
    pub unsafe fn draw(&mut self, rects: &[Rect<f64>], to_glwindow: AffineMap<f64>, time: f64) {
        let mut vertices: Vec<SelectionVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for rect in rects {
            for arrow in rect.ccw_side_arrows() {
                for corner in arrow.corners() {
                    let vertex = SelectionVertex {
                        position: (to_glwindow * corner).cwise_into_lossy().to_array(),
                    };

                    indices.push(vertices.len() as u32);
                    vertices.push(vertex);
                }
            }
        }

        // Draw call
        self.gl.enable(glow::BLEND);
        self.gl.blend_func(glow::ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        self.gl.blend_equation(glow::FUNC_ADD);

        self.vertex_array.bind();
        self.array_buffer.buffer_data(&vertices);
        self.element_buffer.buffer_data(&indices);

        self.shader.use_program();

        self.shader.uniform("time", time);

        self.gl
            .draw_elements(glow::LINES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }
}
