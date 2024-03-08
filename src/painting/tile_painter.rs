use std::{mem::size_of, sync::Arc};

use glow::HasContext;
use memoffset::offset_of;

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
    pub texture_rect: Rect<usize>,
    pub corners: [Point<f64>; 4],
}

pub struct RectPainter {
    shader: Shader,
    array_buffer: GlBuffer<TileVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
    gl: Arc<glow::Context>,
}

impl RectPainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        let vs_source = include_str!("shaders/tile.vert");
        let fs_source = include_str!("shaders/tile.frag");
        let shader = Shader::from_source(gl.clone(), &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl.clone(), GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArray::new(gl.clone());

        vertex_array.bind();
        array_buffer.bind();
        element_buffer.bind();

        let size = size_of::<TileVertex>();
        shader.assign_attribute_f32(
            "in_glwindow_position",
            &VertexAttribDesc::VEC2,
            offset_of!(TileVertex, position) as i32,
            size as i32,
        );
        shader.assign_attribute_f32(
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
            gl,
        }
    }

    /// Why not directly take draw_tiles in the Glwindow frame? Because to_glwindow can be any
    /// AffineMap, it does not have to map axis aligned rectangles to axis aligned rectangles.
    pub unsafe fn draw(
        &mut self,
        draw_rects: &[DrawRect],
        texture: &GlTexture,
        to_glwindow: AffineMap<f64>,
    ) {
        // Create list of vertices for draw_tiles with texture coordinates from atlas
        // Draw two triangles per tile
        let mut vertices: Vec<TileVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (i_tile, tile) in draw_rects.iter().enumerate() {
            let atlas_corners = tile.texture_rect.corners();

            for (tile_corner, atlas_corner) in tile.corners.into_iter().zip(atlas_corners) {
                let vertex = TileVertex {
                    texcoord: (texture.bitmap_to_gltexture() * atlas_corner.cwise_into_lossy())
                        .cwise_into_lossy()
                        .to_array(),
                    position: (to_glwindow * tile_corner).cwise_into_lossy().to_array(),
                };

                vertices.push(vertex);
            }
            let tile_indices = Rect::<f64>::TRIANGLE_INDICES.map(|i| i + (i_tile as u32) * 4u32);
            indices.extend(tile_indices);
        }

        // Draw call
        self.gl.enable(glow::BLEND);
        self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        self.gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        self.gl.active_texture(glow::TEXTURE0);
        self.gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));

        self.vertex_array.bind();
        self.array_buffer.buffer_data(&vertices);
        self.element_buffer.buffer_data(&indices);

        self.shader.use_program();

        self.shader.uniform("tile_atlas_texture", glow::TEXTURE0);

        self.gl
            .draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }
}
