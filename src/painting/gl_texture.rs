use crate::{
    field::{Field, RgbaField},
    math::{affine_map::AffineMap, point::Point, rect::Rect, rgba8::Rgba8},
    painting::gl_garbage::{GlResource, gl_release},
};
use bytemuck::{Pod, cast_slice};
use glow::{HasContext, PixelUnpackData};

pub struct GlTexture {
    pub id: glow::Texture,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum Filter {
    Linear = glow::LINEAR,
    Nearest = glow::NEAREST,
}

#[derive(Debug, Clone, Copy)]
pub enum TextureFormat {
    SRGBA8,
    R16U,
    R8,
}

impl TextureFormat {
    pub fn internal_format(self) -> u32 {
        match self {
            Self::SRGBA8 => glow::SRGB8_ALPHA8,
            Self::R16U => glow::R16UI,
            Self::R8 => glow::R8,
        }
    }

    pub fn format(self) -> u32 {
        match self {
            Self::SRGBA8 => glow::RGBA,
            Self::R16U => glow::RED_INTEGER,
            Self::R8 => glow::RED,
        }
    }

    /// type
    pub fn ty(self) -> u32 {
        match self {
            Self::SRGBA8 => glow::UNSIGNED_BYTE,
            Self::R16U => glow::UNSIGNED_SHORT,
            Self::R8 => glow::UNSIGNED_BYTE,
        }
    }
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
        texture.texture_image_srgba8(gl, bitmap);
        texture
    }

    unsafe fn texture_image_raw<T: Pod>(
        &mut self,
        gl: &glow::Context,
        format: TextureFormat,
        bitmap: &Field<T>,
    ) {
        assert_eq!(bitmap.width(), self.width);
        assert_eq!(bitmap.height(), self.height);

        let bitmap_bytes: &[u8] = cast_slice(bitmap.as_slice());

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            format.internal_format() as i32,
            bitmap.width() as i32,
            bitmap.height() as i32,
            0,
            format.format(),
            format.ty(),
            PixelUnpackData::Slice(Some(bitmap_bytes)),
        );
    }

    pub unsafe fn texture_image_srgba8(&mut self, gl: &glow::Context, bitmap: &Field<Rgba8>) {
        self.texture_image_raw(gl, TextureFormat::SRGBA8, bitmap);
    }

    pub unsafe fn texture_image_red_u16(&mut self, gl: &glow::Context, gray: &Field<u16>) {
        self.texture_image_raw(gl, TextureFormat::R16U, gray)
    }

    pub unsafe fn texture_image_red8(&mut self, gl: &glow::Context, gray: &Field<u8>) {
        self.texture_image_raw(gl, TextureFormat::R8, gray)
    }

    pub unsafe fn texture_sub_image_raw<T: Pod>(
        &mut self,
        gl: &glow::Context,
        format: TextureFormat,
        bitmap_rect: Rect<i64>,
        texture_rect: Rect<i64>,
        field: &Field<T>,
    ) {
        assert_eq!(bitmap_rect.size(), texture_rect.size());
        if bitmap_rect.is_empty() {
            return;
        }
        assert!(field.bounds().contains_rect(bitmap_rect));

        let bitmap_offset = field.linear_index(bitmap_rect.top_left()).unwrap();
        let bitmap_bytes: &[u8] = cast_slice(&field.linear_slice()[bitmap_offset..]);

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, field.width() as i32);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            texture_rect.left() as i32,
            texture_rect.top() as i32,
            texture_rect.width() as i32,
            texture_rect.height() as i32,
            format.format(),
            format.ty(),
            PixelUnpackData::Slice(Some(bitmap_bytes)),
        );
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
    }

    pub unsafe fn texture_sub_image_srgba8(
        &mut self,
        gl: &glow::Context,
        bitmap_rect: Rect<i64>,
        texture_rect: Rect<i64>,
        field: &Field<Rgba8>,
    ) {
        self.texture_sub_image_raw(gl, TextureFormat::SRGBA8, bitmap_rect, texture_rect, field);
    }

    /// Affine map from bitmap coordinates (0,0 at top left) to Gltexture coordinates.
    pub fn gltexture_from_bitmap(&self) -> AffineMap<f64> {
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
        gl_release(GlResource::Texture(self.id));
    }
}
