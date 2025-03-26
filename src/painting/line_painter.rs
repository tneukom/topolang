use crate::{
    math::{affine_map::AffineMap, arrow::Arrow, rect::Rect},
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArrayObject},
        shader::{Shader, VertexAttribDesc},
    },
};
use glow::HasContext;
use std::mem::{offset_of, size_of};

#[derive(Debug, Clone, Copy)]
struct LineVertex {
    pub position: [f32; 2],
}

pub struct LinePainter {
    shader: Shader,
    array_buffer: GlBuffer<LineVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArrayObject,
}

impl LinePainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/line.vert");
        let fs_source = include_str!("shaders/line.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArrayObject::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);

        let size = size_of::<LineVertex>();
        shader.assign_attribute_f32(
            gl,
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
        }
    }

    pub unsafe fn draw_lines(
        &mut self,
        gl: &glow::Context,
        lines: &[Arrow<f64>],
        to_device: AffineMap<f64>,
        time: f64,
    ) {
        let mut vertices: Vec<LineVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for line in lines {
            for corner in line.corners() {
                let vertex = LineVertex {
                    position: (to_device * corner).cwise_as().to_array(),
                };

                indices.push(vertices.len() as u32);
                vertices.push(vertex);
            }
        }

        // Draw call
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        self.vertex_array.bind(gl);
        self.array_buffer.buffer_data(gl, &vertices);
        self.element_buffer.buffer_data(gl, &indices);

        self.shader.use_program(gl);

        self.shader.uniform(gl, "time", time);

        gl.draw_elements(glow::LINES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }

    pub unsafe fn draw_rect(
        &mut self,
        gl: &glow::Context,
        rect: Rect<f64>,
        to_device: AffineMap<f64>,
        time: f64,
    ) {
        let sides = rect.ccw_side_arrows();
        self.draw_lines(gl, &sides, to_device, time);
    }
}
