use crate::{field::Field, math::affine_map::AffineMap, painting::sprite_painter::SpritePainter};
use std::sync::Arc;

pub struct Glow {
    pub outline: Arc<Field<u8>>,
    pub alpha: f64,
}

pub struct GlowPainter {
    sprite_painter: SpritePainter,
}

impl GlowPainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/glow.vert");
        let fs_source = include_str!("shaders/glow.frag");

        Self {
            sprite_painter: SpritePainter::new(gl, vs_source, fs_source),
        }
    }

    pub unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        glow: &Glow,
        world_to_view: AffineMap<f64>,
        view_to_device: AffineMap<f64>,
    ) {
        self.sprite_painter.setup_draw_red8(
            gl,
            glow.outline.clone(),
            world_to_view,
            view_to_device,
        );

        self.sprite_painter
            .shader
            .uniform(gl, "glow_alpha", glow.alpha as f32);

        self.sprite_painter.draw(gl);
    }
}
