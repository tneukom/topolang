use crate::{
    bitmap::Bitmap,
    math::{
        affine_map::AffineMap,
        generic::{CwiseEuclidDivRem, EuclidDivRem},
        point::Point,
        rect::Rect,
        rgba8::Rgba8,
    },
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArray},
        gl_texture::{Filter, GlTexture},
        shader::{Shader, VertexAttribDesc},
    },
    pixmap::Pixmap,
};
use glow::HasContext;
use memoffset::offset_of;
use std::{collections::HashMap, fs::read_to_string, mem::size_of, sync::Arc};

#[derive(Debug, Clone, Copy)]
pub struct TileVertex {
    pub position: [f32; 2],
    pub texcoord: [f32; 2],
}

pub struct TileAtlas {
    bitmap: Bitmap,
    tile_size: usize,
    rows: usize,
    columns: usize,

    /// Number of tiles already added
    size: usize,
}

impl TileAtlas {
    pub fn new(tile_size: usize, rows: usize, columns: usize) -> Self {
        Self {
            bitmap: Bitmap::plain(columns * tile_size, rows * tile_size, Rgba8::TRANSPARENT),
            tile_size,
            rows,
            columns,
            size: 0,
        }
    }

    pub fn bitmap(&self) -> &Bitmap {
        &self.bitmap
    }

    pub fn add(&mut self, tile: &Bitmap) -> Rect<usize> {
        assert!(self.size < self.rows * self.columns);

        let column = self.size % self.columns;
        let row = self.size / self.columns;

        let tile_pos = Point::new(column, row) * self.tile_size;
        self.bitmap.blit(tile, tile_pos);
        self.size += 1;

        Rect::low_size(tile_pos, Point::new(self.tile_size, self.tile_size))
    }
}

/// Draw a single tile at a given position and size
pub struct DrawTile {
    pub atlas_rect: Rect<usize>,
    pub corners: [Point<f64>; 4],
}

pub struct TilePainter {
    shader: Shader,
    texture: GlTexture,
    array_buffer: GlBuffer<TileVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArray,
    gl: Arc<glow::Context>,
}

impl TilePainter {
    const TILE_SIZE: usize = 8;
    const ATLAS_SIZE: usize = 32; // Number of rows and columns in in the atlas
    const ATLAS_RESOLUTION: usize = Self::TILE_SIZE * Self::ATLAS_SIZE;

    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        let vs_source = read_to_string("resources/shaders/tile.vert").unwrap();
        let fs_source = read_to_string("resources/shaders/tile.frag").unwrap();
        let shader = Shader::from_source(gl.clone(), &vs_source, &fs_source);

        let texture = GlTexture::from_size(
            gl.clone(),
            Self::ATLAS_RESOLUTION,
            Self::ATLAS_RESOLUTION,
            Filter::Nearest,
        );

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
            texture,
            array_buffer,
            element_buffer,
            vertex_array,
            gl,
        }
    }

    pub fn pixmap_draw_tiles(pixmap: &Pixmap) -> (Bitmap, Vec<DrawTile>) {
        // split pixels into 8x8 tiles
        let mut tiles: HashMap<Point<i64>, Bitmap> = HashMap::new();
        for (&pixel, &color) in pixmap {
            if color == Rgba8::TRANSPARENT {
                continue;
            }

            let tile_index = pixel.point().euclid_div(Self::TILE_SIZE as i64);
            let tile = tiles
                .entry(tile_index)
                .or_insert_with(|| Bitmap::transparent(Self::TILE_SIZE, Self::TILE_SIZE));

            let pixel_in_tile: Point<usize> = pixel
                .point()
                .euclid_rem(Self::TILE_SIZE as i64)
                .cwise_try_into()
                .unwrap();
            tile[pixel_in_tile] = color;
        }

        // create tile atlas and draw calls
        let mut draw_tiles = Vec::new();
        let mut atlas = TileAtlas::new(Self::TILE_SIZE, Self::ATLAS_SIZE, Self::ATLAS_SIZE);
        for (&tile_index, tile) in &tiles {
            let atlas_rect = atlas.add(tile);

            let world_tile_rect = Rect::low_size(tile_index, [1, 1]) * (Self::TILE_SIZE as i64);
            let draw_tile = DrawTile {
                atlas_rect,
                corners: world_tile_rect.cwise_into_lossy().corners(),
            };
            draw_tiles.push(draw_tile);
        }

        (atlas.bitmap, draw_tiles)
    }

    /// Why not directly take draw_tiles in the Glwindow frame? Because to_glwindow can be any
    /// AffineMap, it does not have to map axis aligned rectangles to axis aligned rectangles.
    pub unsafe fn draw(
        &mut self,
        draw_tiles: &Vec<DrawTile>,
        atlas: &Bitmap,
        to_glwindow: AffineMap<f64>,
    ) {
        // Create list of vertices for draw_tiles with texture coordinates from atlas
        // Draw two triangles per tile
        let mut vertices: Vec<TileVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (i_tile, tile) in draw_tiles.iter().enumerate() {
            // HACK: The padding makes sure that tiles are properly rendered even if the vertices
            // lie exactly on pixel centers. This seems to cause the UV coordinate to lie outside
            // of the texture rectangle. So we make texture rectangle smaller by a very small
            // amount.
            let atlas_corners = tile.atlas_rect.corners();

            for (tile_corner, atlas_corner) in tile.corners.into_iter().zip(atlas_corners) {
                let vertex = TileVertex {
                    texcoord: (self.texture.bitmap_to_gltexture()
                        * atlas_corner.cwise_into_lossy())
                    .cwise_into_lossy()
                    .to_array(),
                    position: (to_glwindow * tile_corner).cwise_into_lossy().to_array(),
                };

                vertices.push(vertex);
            }
            let tile_indices = Rect::<f64>::TRIANGLE_INDICES.map(|i| i + (i_tile as u32) * 4u32);
            indices.extend(tile_indices);
        }

        // println!("vertices: {vertices:?}");

        // Draw call
        self.gl.enable(glow::BLEND);
        self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        self.gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        self.gl.active_texture(glow::TEXTURE0);
        self.gl
            .bind_texture(glow::TEXTURE_2D, Some(self.texture.id));

        // Upload atlas bitmap to self.texture
        self.texture.texture_image(&atlas);

        self.vertex_array.bind();
        self.array_buffer.buffer_data(&vertices);
        self.element_buffer.buffer_data(&indices);

        self.shader.use_program();

        self.shader.uniform("tile_atlas_texture", glow::TEXTURE0);

        self.gl
            .draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);
    }
}
