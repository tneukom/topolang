use crate::{
    math::{affine_map::AffineMap, arrow::Arrow, rect::Rect},
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
        shader::{Shader, VertexAttribDesc},
    },
};
use glow::HasContext;
use std::{
    mem::{offset_of, size_of},
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
struct LineVertex {
    pub position: [f32; 2],
}

pub struct LinePainter {
    shader: Shader,
    array_buffer: GlBuffer<LineVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
    gl: Arc<glow::Context>,
}

impl LinePainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        let vs_source = include_str!("shaders/line.vert");
        let fs_source = include_str!("shaders/line.frag");
        let shader = Shader::from_source(gl.clone(), &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl.clone());

        vertex_array.bind();
        array_buffer.bind();
        element_buffer.bind();

        let size = size_of::<LineVertex>();
        shader.assign_attribute_f32(
            "in_device_position",
            &VertexAttribDesc::VEC2,
            offset_of!(LineVertex, position) as i32,
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

    pub unsafe fn draw_lines(
        &mut self,
        lines: &[Arrow<f64>],
        to_device: AffineMap<f64>,
        time: f64,
    ) {
        let mut vertices: Vec<LineVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for line in lines {
            for corner in line.corners() {
                let vertex = LineVertex {
                    position: (to_device * corner).cwise_cast().to_array(),
                };

                indices.push(vertices.len() as u32);
                vertices.push(vertex);
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

    pub unsafe fn draw_rect(&mut self, rect: Rect<f64>, to_device: AffineMap<f64>, time: f64) {
        let sides = rect.ccw_side_arrows();
        self.draw_lines(&sides, to_device, time);
    }
}
