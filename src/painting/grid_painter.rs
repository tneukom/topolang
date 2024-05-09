use std::{mem::size_of, sync::Arc};

use glow::HasContext;
use memoffset::offset_of;

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

    gl: Arc<glow::Context>,
}

impl GridPainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> GridPainter {
        let vs_source = include_str!("shaders/grid.vert");
        let fs_source = include_str!("shaders/grid.frag");
        let shader = Shader::from_source(gl.clone(), &vs_source, &fs_source);

        let array_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl.clone());

        vertex_array.bind();
        array_buffer.bind();
        element_buffer.bind();

        shader.assign_attribute_f32(
            "in_device_position",
            &VertexAttribDesc::VEC2,
            offset_of!(GridVertex, position) as i32,
            size_of::<GridVertex>() as i32,
        );

        let rect = Rect::<f64>::low_size(Point(-1.0, -1.0), Point(2.0, 2.0));
        let vertices = rect.corners().map(|corner| GridVertex {
            position: corner.cwise_into_lossy().to_array(),
        });

        array_buffer.buffer_data(&vertices);
        element_buffer.buffer_data(&Rect::<f64>::TRIANGLE_INDICES);

        GridPainter {
            shader,
            array_buffer,
            element_buffer,
            vertex_array,
            gl: gl,
        }
    }

    // Draw a grid, the lines are 1 pixel wide.
    // The offset and spacing are in the Glpixel coordinate system (pixel with origin at bottom left of the screen)
    pub unsafe fn draw(&self, offset: Point<f64>, spacing: Point<f64>, frames: &CoordinateFrames) {
        self.gl.enable(glow::BLEND);
        self.gl
            .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        self.gl.blend_equation(glow::FUNC_ADD);

        self.vertex_array.bind();
        self.shader.use_program();
        self.shader.uniform("offset", offset);
        self.shader.uniform("spacing", spacing);
        let mat_device_to_window = Matrix3::from(frames.device_to_window());
        self.shader
            .uniform("device_to_window", &mat_device_to_window);

        self.gl
            .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0)
    }
}
