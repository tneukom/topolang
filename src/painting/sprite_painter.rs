use crate::{
    field::Field,
    math::{affine_map::AffineMap, matrix3::Matrix3, rgba8::Rgba8},
    painting::{
        gl_buffer::GlVertexArrayObject,
        gl_texture::{Filter, GlTexture},
        rect_vertices::RectVertices,
        shader::Shader,
    },
};
use glow::HasContext;
use std::{
    any::Any,
    sync::{Arc, Weak},
};
use weak_table::PtrWeakKeyHashMap;

/// Paint a Field<T> at the field bounds in world coordinates using a custom shader. The textures
/// for the passed fields are cached using a weak key.
pub struct SpritePainter {
    position_vertices: RectVertices,
    texcoord_vertices: RectVertices,
    vao: GlVertexArrayObject,

    pub shader: Shader,
    textures: PtrWeakKeyHashMap<Weak<dyn Any + Send + Sync>, GlTexture>,
}

impl SpritePainter {
    pub unsafe fn new(gl: &glow::Context, vs_source: &str, fs_source: &str) -> Self {
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
            position_vertices,
            texcoord_vertices,
            vao,
            textures: PtrWeakKeyHashMap::default(),
        }
    }

    pub unsafe fn setup_draw_srgba8(
        &mut self,
        gl: &glow::Context,
        sprite: Arc<Field<Rgba8>>,
        world_to_view: AffineMap<f64>,
        view_to_device: AffineMap<f64>,
    ) {
        self.setup_draw(
            gl,
            sprite,
            GlTexture::texture_image_srgba8,
            world_to_view,
            view_to_device,
        );
    }

    pub unsafe fn setup_draw_red8(
        &mut self,
        gl: &glow::Context,
        sprite: Arc<Field<u8>>,
        world_to_view: AffineMap<f64>,
        view_to_device: AffineMap<f64>,
    ) {
        self.setup_draw(
            gl,
            sprite,
            GlTexture::texture_image_red8,
            world_to_view,
            view_to_device,
        );
    }

    /// Draw whole texture as a single rectangle
    unsafe fn setup_draw<T: 'static + Send + Sync>(
        &mut self,
        gl: &glow::Context,
        sprite: Arc<Field<T>>,
        texture_upload: unsafe fn(&mut GlTexture, &glow::Context, &Field<T>),
        world_to_view: AffineMap<f64>,
        view_to_device: AffineMap<f64>,
    ) {
        self.textures.remove_expired();

        let key = sprite.clone() as Arc<dyn Any + Send + Sync>;
        let texture = self.textures.entry(key).or_insert_with(|| {
            let mut texture =
                GlTexture::from_size(gl, sprite.size().x, sprite.size().y, Filter::Nearest);
            texture_upload(&mut texture, gl, &sprite);
            texture
        });

        // Update vertices
        let world_rect = sprite.bounds();
        self.position_vertices.update(gl, world_rect.cwise_as());
        let bitmap_rect = world_rect - world_rect.low();
        self.texcoord_vertices.update(gl, bitmap_rect.cwise_as());

        // Alpha blending with premultiplied alpha
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));

        self.vao.bind(gl);
        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "alpha_texture", 0i32);

        let world_to_device = view_to_device * world_to_view;
        let mat_world_to_device = Matrix3::from(world_to_device);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        let bitmap_to_gltexture = texture.bitmap_to_gltexture();
        let mat_bitmap_to_gltexture = Matrix3::from(bitmap_to_gltexture);
        self.shader
            .uniform(gl, "bitmap_to_gltexture", &mat_bitmap_to_gltexture);
    }

    pub unsafe fn draw(&self, gl: &glow::Context) {
        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
