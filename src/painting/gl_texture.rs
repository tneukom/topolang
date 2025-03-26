use crate::{
    field::{Field, RgbaField},
    math::{affine_map::AffineMap, point::Point, rgba8::Rgba8},
};
use glow::{HasContext, PixelUnpackData};
use log::warn;

pub struct GlTexture {
    pub id: glow::Texture,
    pub width: i64,
    pub height: i64,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Filter {
    Linear = glow::LINEAR,
    Nearest = glow::NEAREST,
}

impl GlTexture {
    pub unsafe fn from_size(gl: &glow::Context, width: i64, height: i64, filter: Filter) -> Self {
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

        GlTexture { id, width, height }
    }

    /// Bitmap colorspace is assumed to be SRGB
    pub unsafe fn from_bitmap(gl: &glow::Context, bitmap: &RgbaField, filter: Filter) -> Self {
        let mut texture = Self::from_size(gl, bitmap.width(), bitmap.height(), filter);
        texture.texture_image(gl, bitmap);
        texture
    }

    pub unsafe fn texture_image(&mut self, gl: &glow::Context, bitmap: &Field<Rgba8>) {
        assert_eq!(bitmap.width(), self.width);
        assert_eq!(bitmap.height(), self.height);

        let bitmap_bytes = bitmap.linear_slice().align_to::<u8>().1;

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::SRGB8_ALPHA8 as i32, // internal_format, see notes/srgb.md
            // glow::RGBA8 as i32, // internal_format
            bitmap.width() as i32,
            bitmap.height() as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(bitmap_bytes)),
        );
    }

    pub unsafe fn texture_sub_image(
        &mut self,
        gl: &glow::Context,
        offset: Point<i64>,
        field: &Field<Rgba8>,
    ) {
        let bitmap_bytes = field.linear_slice().align_to::<u8>().1;

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            offset.x as i32,
            offset.y as i32,
            field.width() as i32,
            field.height() as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(bitmap_bytes)),
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

    pub fn size(&self) -> Point<i64> {
        Point(self.width, self.height)
    }
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        warn!("Leaking GlTexture");
        // unsafe {
        //     self.context.delete_texture(self.id);
        // }
    }
}
