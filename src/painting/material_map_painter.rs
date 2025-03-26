use crate::{
    field::RgbaField,
    math::{affine_map::AffineMap, matrix3::Matrix3, point::Point, rect::Rect, rgba8::Rgba8},
    painting::{
        gl_buffer::GlVertexArrayObject,
        gl_texture::{Filter, GlTexture},
        rect_vertices::RectVertices,
        shader::Shader,
    },
};
use glow::HasContext;

pub struct RgbaFieldPainter {
    position_vertices: RectVertices,
    texcoord_vertices: RectVertices,
    vao: GlVertexArrayObject,
    shader: Shader,
    texture: Option<(GlTexture, Point<i64>)>,
}

impl RgbaFieldPainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/material.vert");
        let fs_source = include_str!("shaders/material.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        let position_vertices = RectVertices::new(gl);
        let texcoord_vertices = RectVertices::new(gl);

        let vao = GlVertexArrayObject::new(gl);
        vao.bind(gl);

        // Update position and texcoord vertices
        position_vertices.assign_attribute(gl, &shader, "in_world_position");

        texcoord_vertices.assign_attribute(gl, &shader, "in_bitmap_uv");

        Self {
            shader,
            texture: None,
            position_vertices,
            texcoord_vertices,
            vao,
        }
    }

    fn next_power_of_two(n: i64) -> i64 {
        assert!(n >= 0);
        let mut power = 1;
        while power <= n {
            power *= 2;
        }
        power
    }

    /// Draw whole texture as a single rectangle
    pub unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        rgba_field: &RgbaField,
        world_to_device: AffineMap<f64>,
    ) {
        if !rgba_field.bounds().has_positive_area() {
            println!("Trying to draw empty field.");
            return;
        }

        // If rgba_field size is different from texture size, reset texture
        // REVISIT: Could we just reset the texture if the new size is larger than the old? Could
        //   cause problems with mipmaps, wasted memory
        if let Some((_, bitmap_size)) = &self.texture {
            if *bitmap_size != rgba_field.size() {
                self.texture = None;
            }
        }

        // New texture in case it is None
        let (texture, bitmap_size) = self.texture.get_or_insert_with(|| {
            // WebGL support for non-power of 2 doesn't seem to be very good
            // See https://www.khronos.org/webgl/wiki/WebGL_and_OpenGL_Differences#Non-Power_of_Two_Texture_Support
            let texture_width = Self::next_power_of_two(rgba_field.width());
            let texture_height = Self::next_power_of_two(rgba_field.height());
            let texture_size = Point(texture_width, texture_height);

            // TODO: Check mipmap generation
            let mut texture =
                GlTexture::from_size(gl, texture_width, texture_height, Filter::Nearest);
            let transparent = RgbaField::filled(
                Rect::low_size(Point::ZERO, texture_size),
                Rgba8::TRANSPARENT,
            );
            texture.texture_image(gl, &transparent);
            (texture, rgba_field.size())
        });

        // Upload texture data
        texture.texture_sub_image(gl, Point::ZERO, &rgba_field);

        let texture_rect = Rect::low_size(Point::ZERO, *bitmap_size);
        let rect = texture_rect + rgba_field.bounds().low();

        // Alpha blending with premultiplied alpha
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));

        self.position_vertices.update(gl, rect.cwise_as());
        self.texcoord_vertices.update(gl, texture_rect.cwise_as());

        self.vao.bind(gl);
        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "material_texture", glow::TEXTURE0);

        let mat_world_to_device = Matrix3::from(world_to_device);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        let bitmap_to_gltexture = texture.bitmap_to_gltexture();
        let mat_bitmap_to_gltexture = Matrix3::from(bitmap_to_gltexture);
        self.shader
            .uniform(gl, "bitmap_to_gltexture", &mat_bitmap_to_gltexture);

        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
