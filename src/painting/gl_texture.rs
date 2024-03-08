use crate::{
    bitmap::Bitmap,
    math::{affine_map::AffineMap, point::Point},
};
use glow::{HasContext, PixelUnpackData};
use std::sync::Arc;

pub struct GlTexture {
    pub context: Arc<glow::Context>,
    pub id: glow::Texture,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Filter {
    Linear = glow::LINEAR,
    Nearest = glow::NEAREST,
}

impl GlTexture {
    pub unsafe fn from_size(
        gl: Arc<glow::Context>,
        width: usize,
        height: usize,
        filter: Filter,
    ) -> Self {
        let id = gl.create_texture().expect("Failed to create texture");

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(id));

        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );

        GlTexture {
            context: gl,
            id,
            width,
            height,
        }
    }

    /// Bitmap colorspace is assumed to be SRGB
    pub unsafe fn from_bitmap(
        context: Arc<glow::Context>,
        bitmap: &Bitmap,
        filter: Filter,
    ) -> Self {
        let mut texture = Self::from_size(context.clone(), bitmap.width(), bitmap.height(), filter);
        texture.texture_image(bitmap);
        texture
    }

    pub unsafe fn texture_image(&mut self, bitmap: &Bitmap) {
        assert_eq!(bitmap.width(), self.width);
        assert_eq!(bitmap.height(), self.height);

        let bitmap_bytes = bitmap.linear_slice().align_to::<u8>().1;

        self.context.active_texture(glow::TEXTURE0);
        self.context.bind_texture(glow::TEXTURE_2D, Some(self.id));
        self.context.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            // glow::SRGB8_ALPHA8 as i32, see notes/srgb.md
            glow::RGBA8 as i32,
            bitmap.width() as i32,
            bitmap.height() as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(bitmap_bytes),
        );
    }

    pub unsafe fn texture_sub_image(&mut self, offset: Point<usize>, bitmap: &Bitmap) {
        let bitmap_bytes = bitmap.linear_slice().align_to::<u8>().1;

        self.context.active_texture(glow::TEXTURE0);
        self.context.bind_texture(glow::TEXTURE_2D, Some(self.id));
        self.context.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            offset.x as i32,
            offset.y as i32,
            bitmap.width() as i32,
            bitmap.height() as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(bitmap_bytes),
        );
    }

    /// Affine map from bitmap coordinates (0,0 at top left) to Gltexture coordinates.
    pub fn bitmap_to_gltexture(&self) -> AffineMap<f64> {
        AffineMap::map_points(
            Point(0.0, 0.0),
            Point(0.0, 0.0),
            Point(self.width as f64, 0.0),
            Point(1.0, 0.0),
            Point(0.0, self.height as f64),
            Point(0.0, 1.0),
        )
    }

    pub fn size(&self) -> Point<usize> {
        Point(self.width, self.height)
    }
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        unsafe {
            self.context.delete_texture(self.id);
        }
    }
}
