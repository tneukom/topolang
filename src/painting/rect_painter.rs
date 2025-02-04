use std::mem::{offset_of, size_of};

use glow::HasContext;

use crate::{
    math::{affine_map::AffineMap, point::Point, rect::Rect},
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
        gl_texture::GlTexture,
        shader::{Shader, VertexAttribDesc},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct TileVertex {
    pub position: [f32; 2],
    pub texcoord: [f32; 2],
}

/// Draw a single tile at a given position and size
pub struct DrawRect {
    pub texture_rect: Rect<i64>,
    pub corners: [Point<f64>; 4],
}

/// Paint a textured rectangle
pub struct FillRectPainter {
    shader: Shader,
    array_buffer: GlBuffer<TileVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
}

impl FillRectPainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/fill_rect.vert");
        let fs_source = include_str!("shaders/fill_rect.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);

        let size = size_of::<TileVertex>();
        shader.assign_attribute_f32(
            gl,
            "in_device_position",
            &VertexAttribDesc::VEC2,
            offset_of!(TileVertex, position) as i32,
            size as i32,
        );
        shader.assign_attribute_f32(
            gl,
            "in_texcoord",
            &VertexAttribDesc::VEC2,
            offset_of!(TileVertex, texcoord) as i32,
            size as i32,
        );

        Self {
            shader,
            array_buffer,
            element_buffer,
            vertex_array,
        }
    }

    /// Why not directly take draw_tiles in the device frame? Because to_device can be any
    /// AffineMap, it does not have to map axis aligned rectangles to axis aligned rectangles.
    pub unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        draw_rects: &[DrawRect],
        texture: &GlTexture,
        to_device: AffineMap<f64>,
        time: f64,
    ) {
        // Create list of vertices for draw_tiles with texture coordinates from atlas
        // Draw two triangles per tile
        let mut vertices: Vec<TileVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (i_tile, tile) in draw_rects.iter().enumerate() {
            let atlas_corners = tile.texture_rect.corners();

            for (tile_corner, atlas_corner) in tile.corners.into_iter().zip(atlas_corners) {
                let vertex = TileVertex {
                    texcoord: (texture.bitmap_to_gltexture() * atlas_corner.cwise_as())
                        .cwise_as()
                        .to_array(),
                    position: (to_device * tile_corner).cwise_as().to_array(),
                };

                vertices.push(vertex);
            }
            let tile_indices = Rect::<f64>::TRIANGLE_INDICES.map(|i| i + (i_tile as u32) * 4u32);
            indices.extend(tile_indices);
        }

        // Alpha blending with premultiplied alpha
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));

        self.vertex_array.bind(gl);
        self.array_buffer.buffer_data(gl, &vertices);
        self.element_buffer.buffer_data(gl, &indices);

        self.shader.use_program(gl);

        self.shader
            .uniform(gl, "tile_atlas_texture", glow::TEXTURE0);
        self.shader.uniform(gl, "time", time);

        gl.draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }
}
