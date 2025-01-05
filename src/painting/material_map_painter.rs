use std::sync::Arc;

use crate::{
    field::RgbaField,
    material::Material,
    math::{affine_map::AffineMap, point::Point, rect::Rect, rgba8::Rgba8},
    painting::{
        gl_texture::{Filter, GlTexture},
        rect_painter::{DrawRect, RectPainter},
    },
    pixmap::MaterialMap,
};

pub struct RgbaFieldPainter {
    rect_painter: RectPainter,
    texture: Option<(GlTexture, Point<i64>)>,
    gl: Arc<glow::Context>,
}

impl RgbaFieldPainter {
    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        Self {
            texture: None,
            rect_painter: RectPainter::new(gl.clone()),
            gl: gl,
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
    pub unsafe fn draw(&mut self, rgba_field: &RgbaField, to_device: AffineMap<f64>, time: f64) {
        if !rgba_field.bounds().has_positive_area() {
            println!("Trying to draw empty field.");
            return;
        }

        // If rgba_field size is different from texture size, reset texture
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
            let mut texture = GlTexture::from_size(
                self.gl.clone(),
                texture_width,
                texture_height,
                Filter::Nearest,
            );
            let transparent = RgbaField::filled(
                Rect::low_size(Point::ZERO, texture_size),
                Rgba8::TRANSPARENT,
            );
            texture.texture_image(&transparent);
            (texture, rgba_field.size())
        });

        texture.texture_sub_image(Point::ZERO, &rgba_field);

        let texture_rect = Rect::low_size(Point::ZERO, *bitmap_size);
        let rect = texture_rect + rgba_field.bounds().low();
        let draw_tile = DrawRect {
            texture_rect,
            corners: rect.cwise_cast().corners(),
        };

        self.rect_painter
            .draw(&[draw_tile], texture, to_device, time);
    }
}
