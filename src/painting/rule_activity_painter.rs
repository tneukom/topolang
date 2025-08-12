use crate::{
    field::Field,
    math::{affine_map::AffineMap, matrix3::Matrix3, rect::Rect},
    painting::{
        gl_buffer::GlVertexArrayObject,
        gl_texture::{Filter, GlTexture},
        rect_vertices::RectVertices,
        shader::Shader,
    },
    rule_activity_effect::{Glow, RuleActivity},
    topology::ModificationTime,
};
use ahash::HashMap;
use glow::HasContext;

struct Textures {
    bitmap_bounds: Rect<i64>,
    id: GlTexture,
    alpha: GlTexture,
    glows: HashMap<ModificationTime, Glow>,
}

pub struct RuleActivityPainter {
    position_vertices: RectVertices,
    texcoord_vertices: RectVertices,
    vao: GlVertexArrayObject,

    shader: Shader,
    textures: Option<Textures>,
}

impl RuleActivityPainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/glow.vert");
        let fs_source = include_str!("shaders/glow.frag");
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
            textures: None,
        }
    }

    pub unsafe fn update_glows(&mut self, gl: &glow::Context, rule_activity: &RuleActivity) {
        assert_eq!(
            rule_activity.glow_ids.bounds(),
            rule_activity.glow_alphas.bounds()
        );
        let bounds = rule_activity.glow_ids.bounds();

        let mut id_texture =
            GlTexture::from_size(gl, bounds.width(), bounds.height(), Filter::Nearest);
        // let glow_ids = rule_activity.glow_ids.map(|&i| i as u8);
        // id_texture.texture_image_red8(gl, &glow_ids);
        id_texture.texture_image_red_u16(gl, &rule_activity.glow_ids);

        let mut alpha_texture =
            GlTexture::from_size(gl, bounds.width(), bounds.height(), Filter::Nearest);
        alpha_texture.texture_image_red8(gl, &rule_activity.glow_alphas);

        self.textures = Some(Textures {
            id: id_texture,
            alpha: alpha_texture,
            glows: rule_activity.glows.clone(),
            bitmap_bounds: bounds,
        })
    }

    /// Draw whole texture as a single rectangle
    pub unsafe fn draw_glow(
        &mut self,
        gl: &glow::Context,
        rule_modified_time: ModificationTime,
        glow_intensity: f64,
        world_to_view: AffineMap<f64>,
        view_to_device: AffineMap<f64>,
    ) {
        let Some(textures) = &self.textures else {
            return;
        };

        let glow = textures.glows.get(&rule_modified_time).unwrap();

        // Alpha blending with premultiplied alpha
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.blend_equation(glow::FUNC_ADD);

        // Bind texture
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(textures.alpha.id));

        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, Some(textures.id.id));

        let world_rect = glow.bounds;
        self.position_vertices.update(gl, world_rect.cwise_as());
        let bitmap_rect = glow.bounds - textures.bitmap_bounds.low();
        self.texcoord_vertices.update(gl, bitmap_rect.cwise_as());
        println!("world_rect: {world_rect:?}, bitmap_rect: {bitmap_rect:?}");

        self.vao.bind(gl);
        self.shader.use_program(gl);

        // Update uniforms
        self.shader.uniform(gl, "glow_alpha", glow_intensity as f32);
        self.shader.uniform(gl, "glow_id", glow.id as u32);

        self.shader.uniform(gl, "alpha_texture", 0i32);
        self.shader.uniform(gl, "id_texture", 1i32);

        let world_to_device = view_to_device * world_to_view;
        let mat_world_to_device = Matrix3::from(world_to_device);
        self.shader
            .uniform(gl, "world_to_device", &mat_world_to_device);

        let mat_world_to_view = Matrix3::from(world_to_view);
        self.shader.uniform(gl, "world_to_view", &mat_world_to_view);

        let bitmap_to_gltexture = textures.id.bitmap_to_gltexture();
        let mat_bitmap_to_gltexture = Matrix3::from(bitmap_to_gltexture);
        self.shader
            .uniform(gl, "bitmap_to_gltexture", &mat_bitmap_to_gltexture);

        // World coordinates are same bitmap coordinates
        let view_to_gltexture = bitmap_to_gltexture * world_to_view.inv();
        let mat_view_to_gltexture = Matrix3::from(view_to_gltexture);
        self.shader
            .uniform(gl, "view_to_gltexture", &mat_view_to_gltexture);

        // Paint 2 triangles
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
