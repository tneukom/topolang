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
        view_from_world: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
    ) {
        self.setup_draw(
            gl,
            sprite,
            GlTexture::texture_image_srgba8,
            view_from_world,
            device_from_view,
        );
    }

    pub unsafe fn setup_draw_red8(
        &mut self,
        gl: &glow::Context,
        sprite: Arc<Field<u8>>,
        view_from_world: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
    ) {
        self.setup_draw(
            gl,
            sprite,
            GlTexture::texture_image_red8,
            view_from_world,
            device_from_view,
        );
    }

    /// Draw whole texture as a single rectangle
    unsafe fn setup_draw<T: 'static + Send + Sync>(
        &mut self,
        gl: &glow::Context,
        sprite: Arc<Field<T>>,
        texture_upload: unsafe fn(&mut GlTexture, &glow::Context, &Field<T>),
        view_from_world: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
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

        let device_from_world = device_from_view * view_from_world;
        let mat_device_from_world = Matrix3::from(device_from_world);
        self.shader
            .uniform(gl, "device_from_world", &mat_device_from_world);

        let gltexture_from_bitmap = texture.gltexture_from_bitmap();
        let mat_gltexture_from_bitmap = Matrix3::from(gltexture_from_bitmap);
        self.shader
            .uniform(gl, "gltexture_from_bitmap", &mat_gltexture_from_bitmap);
    }

    pub unsafe fn draw(&self, gl: &glow::Context) {
        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
