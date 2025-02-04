use std::mem::{offset_of, size_of};

use glow::HasContext;

use crate::{
    coordinate_frame::CoordinateFrames,
    math::{matrix3::Matrix3, point::Point, rect::Rect},
};

use super::{
    gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
    shader::{Shader, VertexAttribDesc},
};

pub struct GridVertex {
    pub position: [f32; 2],
}

pub struct GridPainter {
    pub shader: Shader,
    pub array_buffer: GlBuffer<GridVertex>,
    pub element_buffer: GlBuffer<u32>,
    pub vertex_array: GlVertexArray,
}

impl GridPainter {
    pub unsafe fn new(gl: &glow::Context) -> GridPainter {
        let vs_source = include_str!("shaders/grid.vert");
        let fs_source = include_str!("shaders/grid.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);

        shader.assign_attribute_f32(
            gl,
            "in_device_position",
            &VertexAttribDesc::VEC2,
            offset_of!(GridVertex, position) as i32,
            size_of::<GridVertex>() as i32,
        );

        let rect = Rect::<f64>::low_size(Point(-1.0, -1.0), Point(2.0, 2.0));
        let vertices = rect.corners().map(|corner| GridVertex {
            position: corner.cwise_as().to_array(),
        });

        array_buffer.buffer_data(gl, &vertices);
        element_buffer.buffer_data(gl, &Rect::<f64>::TRIANGLE_INDICES);

        GridPainter {
            shader,
            array_buffer,
            element_buffer,
            vertex_array,
        }
    }

    // Draw a grid, the lines are 1 pixel wide.
    // The offset and spacing are in the view coordinate system
    pub unsafe fn draw(
        &self,
        gl: &glow::Context,
        offset: Point<f64>,
        spacing: Point<f64>,
        frames: &CoordinateFrames,
    ) {
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        self.vertex_array.bind(gl);
        self.shader.use_program(gl);
        self.shader.uniform(gl, "offset", offset);
        self.shader.uniform(gl, "spacing", spacing);
        let mat_device_to_view: Matrix3<_> = frames.device_to_view().into();
        self.shader
            .uniform(gl, "device_to_view", &mat_device_to_view);

        gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0)
    }
}
