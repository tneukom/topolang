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
    texture: GlTexture,
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

        let texture = Self::transparent_texture(gl, Point(256, 256));

        Self {
            shader,
            texture,
            position_vertices,
            texcoord_vertices,
            vao,
        }
    }

    unsafe fn transparent_texture(gl: &glow::Context, size: Point<i64>) -> GlTexture {
        // WebGL support for non-power of 2 doesn't seem to be very good
        // See https://www.khronos.org/webgl/wiki/WebGL_and_OpenGL_Differences#Non-Power_of_Two_Texture_Support
        let texture_width = Self::next_power_of_two(size.x);
        let texture_height = Self::next_power_of_two(size.y);
        let texture_size = Point(texture_width, texture_height);

        // TODO: Check mipmap generation
        let mut texture = GlTexture::from_size(gl, texture_width, texture_height, Filter::Nearest);
        let transparent = RgbaField::filled(
            Rect::low_size(Point::ZERO, texture_size),
            Rgba8::TRANSPARENT,
        );
        texture.texture_image(gl, &transparent);

        texture
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
        mut expired: Rect<i64>,
        world_to_device: AffineMap<f64>,
    ) {
        expired = expired.intersect(rgba_field.bounds());

        if !rgba_field.bounds().has_positive_area() {
            println!("Trying to draw empty field.");
            return;
        }

        // If rgba_field is bigger than the size of the texture, reset texture.
        if self.texture.width < rgba_field.width() || self.texture.height < rgba_field.height() {
            let texture_size = Point(
                self.texture.width.max(rgba_field.width()),
                self.texture.height.max(rgba_field.height()),
            );
            self.texture = Self::transparent_texture(gl, texture_size);
            expired = rgba_field.bounds();
        }

        // We only need to update the expired part of the image
        self.texture
            .texture_sub_image(gl, expired, expired - rgba_field.low(), &rgba_field);

        let texture_rect = Rect::low_size(Point::ZERO, rgba_field.size());
        let rect = texture_rect + rgba_field.bounds().low();

        // Alpha blending with premultiplied alpha
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.texture.id));

        self.position_vertices.update(gl, rect.cwise_as());
        self.texcoord_vertices.update(gl, texture_rect.cwise_as());

        self.vao.bind(gl);
        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "material_texture", glow::TEXTURE0);

        let mat_world_to_device = Matrix3::from(world_to_device);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        let bitmap_to_gltexture = self.texture.bitmap_to_gltexture();
        let mat_bitmap_to_gltexture = Matrix3::from(bitmap_to_gltexture);
        self.shader
            .uniform(gl, "bitmap_to_gltexture", &mat_bitmap_to_gltexture);

        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
