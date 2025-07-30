use crate::{
    math::{affine_map::AffineMap, matrix3::Matrix3, point::Point},
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArrayObject},
        rect_vertices::RECT_TRIANGLE_INDICES,
        shader::{Shader, VertexAttribDesc},
    },
};
use glow::HasContext;
use itertools::Itertools;
use std::{
    array::from_fn,
    mem::{offset_of, size_of},
};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NiceLineVertex {
    pub line_start: [f32; 2],
    pub line_end: [f32; 2],
    pub quad_corner: u32,
    pub line_width: f32,
    pub arc_length: f32,
}

#[derive(Debug, Clone, Default)]
pub struct NiceLineGeometry {
    pub vertices: Vec<NiceLineVertex>,
    pub indices: Vec<u32>,
}

impl NiceLineGeometry {
    pub fn add_polyline(&mut self, polyline: &[Point<f32>]) {
        let mut arc_length = 0.0f32;

        for (&line_start, &line_end) in polyline.iter().circular_tuple_windows() {
            let line_width = 0.6f32;
            let line_length = (line_end - line_start).norm();
            // Kind of an arbitrary constant since we don't know what reference frame the
            // lines are in.
            assert!(line_length > 1e-10);

            let make_vertex = |quad_corner: usize| NiceLineVertex {
                line_start: line_start.to_array(),
                line_end: line_end.to_array(),
                arc_length,
                line_width,
                quad_corner: quad_corner as u32,
            };

            let indices = RECT_TRIANGLE_INDICES.map(|i| i + self.vertices.len() as u32);
            self.indices.extend(indices);

            let vertices: [_; 4] = from_fn(make_vertex);
            self.vertices.extend(vertices);

            arc_length += line_length;
        }
    }
}

pub struct NiceLinePainter {
    shader: Shader,
    array_buffer: GlBuffer<NiceLineVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArrayObject,
}

impl NiceLinePainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/nice_line.vert");
        let fs_source = include_str!("shaders/nice_line.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArrayObject::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);

        let size = size_of::<NiceLineVertex>();

        shader.assign_attribute_f32(
            gl,
            "in_line_start",
            &VertexAttribDesc::VEC2,
            offset_of!(NiceLineVertex, line_start) as i32,
            size as i32,
        );

        shader.assign_attribute_f32(
            gl,
            "in_line_end",
            &VertexAttribDesc::VEC2,
            offset_of!(NiceLineVertex, line_end) as i32,
            size as i32,
        );

        shader.assign_attribute_f32(
            gl,
            "in_line_width",
            &VertexAttribDesc::FLOAT,
            offset_of!(NiceLineVertex, line_width) as i32,
            size as i32,
        );

        shader.assign_attribute_i32(
            gl,
            "in_quad_corner",
            &VertexAttribDesc::I32,
            offset_of!(NiceLineVertex, quad_corner) as i32,
            size as i32,
        );

        shader.assign_attribute_f32(
            gl,
            "in_arc_length",
            &VertexAttribDesc::FLOAT,
            offset_of!(NiceLineVertex, arc_length) as i32,
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
        geometry: &NiceLineGeometry,
        view_from: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
        time: f64,
    ) {
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // The depth written by the fragment shader is the distance to the line, this way the
        // connection between line segments is properly drawn.
        gl.enable(glow::DEPTH_TEST);

        self.vertex_array.bind(gl);
        self.array_buffer.buffer_data(gl, &geometry.vertices);
        self.element_buffer.buffer_data(gl, &geometry.indices);

        self.shader.use_program(gl);

        let mat_device_from_view = Matrix3::from(device_from_view);
        self.shader
            .uniform(gl, "device_from_view", &mat_device_from_view);

        let mat_view_from = Matrix3::from(view_from);
        self.shader.uniform(gl, "view_from", &mat_view_from);

        self.shader.uniform(gl, "time", time);

        gl.draw_elements(
            glow::TRIANGLES,
            geometry.indices.len() as i32,
            glow::UNSIGNED_INT,
            0,
        );

        gl.disable(glow::DEPTH_TEST);
    }
}
