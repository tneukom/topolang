use crate::{
    material::Material,
    math::{point::Point, rect::Rect, rgba8::Rgba8},
};
use data_encoding::BASE64;
use image::ImageEncoder;
use std::{
    io::Cursor,
    ops::{Index, IndexMut},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<T> {
    bounds: Rect<i64>,
    elems: Vec<T>,
}

pub type RgbaField = Field<Rgba8>;
pub type MaterialField = Field<Material>;

pub fn field_linear_index(bounds: Rect<i64>, index: Point<i64>) -> Option<usize> {
    if !bounds.half_open_contains(index.point()) {
        None
    } else {
        let i = (index.x() - bounds.x.low) + (index.y() - bounds.y.low) * bounds.width();
        Some(i as usize)
    }
}

impl<T> Field<T> {
    pub fn from_linear(bounds: Rect<i64>, elems: Vec<T>) -> Self {
        assert!(bounds.width() >= 0);
        assert!(bounds.height() >= 0);
        assert_eq!(bounds.width() * bounds.height(), elems.len() as i64);
        Self { bounds, elems }
    }

    pub fn from_map(bounds: Rect<i64>, f: impl FnMut(Point<i64>) -> T) -> Self {
        // Meaning bounds.width() > 0 and bounds.height() > 0
        // assert!(!bounds.is_empty());

        let mut elems = Vec::with_capacity(bounds.width() as usize * bounds.height() as usize);
        elems.extend(bounds.iter_indices().map(f));
        Self::from_linear(bounds, elems)
    }

    pub fn from_rows_array<const WIDTH: usize, const HEIGHT: usize>(
        rows: [[T; WIDTH]; HEIGHT],
    ) -> Self {
        // TODO: Transmute and Vec::from?
        let mut elems = Vec::with_capacity(WIDTH * HEIGHT);
        for row in rows {
            elems.extend(row);
        }
        Self::from_linear(
            Rect::low_size(Point::ZERO, Point(WIDTH as i64, HEIGHT as i64)),
            elems,
        )
    }

    pub fn linear_slice(&self) -> &[T] {
        &self.elems
    }

    pub fn low(&self) -> Point<i64> {
        self.bounds.low()
    }

    pub fn high(&self) -> Point<i64> {
        self.bounds.high()
    }

    pub fn bounds(&self) -> Rect<i64> {
        self.bounds
    }

    pub fn size(&self) -> Point<i64> {
        self.bounds.size()
    }

    pub fn is_empty(&self) -> bool {
        self.bounds.has_zero_area()
    }

    /// Always > 0
    pub fn width(&self) -> i64 {
        self.bounds.width()
    }

    /// Always > 0
    pub fn height(&self) -> i64 {
        self.bounds.height()
    }

    pub fn len(&self) -> usize {
        (self.bounds.width() * self.bounds().height()) as usize
    }

    pub fn indices(&self) -> impl Iterator<Item = Point<i64>> + Clone + use<T> {
        self.bounds.iter_indices()
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (Point<i64>, &T)> + Clone {
        self.indices().zip(self.iter())
    }

    pub fn enumerate_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = (Point<i64>, &'a mut T)> + use<'a, T> {
        let indices = self.indices();
        indices.zip(self.iter_mut())
    }

    pub fn contains_index(&self, index: impl FieldIndex) -> bool {
        self.bounds.contains(index.point())
    }

    pub fn linear_index(&self, index: impl FieldIndex) -> Option<usize> {
        field_linear_index(self.bounds, index.point())
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &T> + Clone {
        self.elems.iter()
    }

    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut T> {
        self.elems.iter_mut()
    }

    pub fn as_slice(&self) -> &[T] {
        self.elems.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.elems.as_mut_slice()
    }

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let i = self.linear_index(index)?;
        Some(&self.elems[i])
    }

    pub fn get_mut(&mut self, index: impl FieldIndex) -> Option<&mut T> {
        let i = self.linear_index(index)?;
        Some(&mut self.elems[i])
    }

    pub fn set(&mut self, index: impl FieldIndex, value: T) -> T {
        let i = self.linear_index(index).unwrap();
        std::mem::replace(&mut self.elems[i], value)
    }

    /// If index is not contained, return `Err(value)` otherwise replace `self[index]` and return
    /// the replaced value.
    pub fn try_set(&mut self, index: impl FieldIndex, value: T) -> Result<T, T> {
        if let Some(i) = self.linear_index(index) {
            Ok(std::mem::replace(&mut self.elems[i], value))
        } else {
            Err(value)
        }
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            bounds: self.bounds + offset,
            elems: self.elems,
        }
    }

    pub fn map<S>(&self, f: impl FnMut(&T) -> S) -> Field<S> {
        let elems = self.elems.iter().map(f).collect();
        Field {
            bounds: self.bounds,
            elems,
        }
    }

    // TODO: Do we need map_with_index and map?
    pub fn map_with_index<S>(&self, mut f: impl FnMut(Point<i64>, &T) -> S) -> Field<S> {
        let elems = self
            .enumerate()
            .map(|(index, value)| f(index, value))
            .collect();
        Field {
            bounds: self.bounds,
            elems,
        }
    }

    pub fn into_map<S>(self, f: impl FnMut(T) -> S) -> Field<S> {
        // Will this reuse the Vec storage if sizeof(S) == sizeof(T)? See
        // https://stackoverflow.com/a/48309116
        // https://www.reddit.com/r/rust/comments/16hx79e/when_does_vecinto_itermapcollect_reallocate_and/
        // https://doc.rust-lang.org/nightly/src/alloc/vec/in_place_collect.rs.html
        let elems = self.elems.into_iter().map(f).collect();
        Field {
            bounds: self.bounds,
            elems,
        }
    }

    pub fn map_into<S: From<T>>(self) -> Field<S> {
        self.into_map(|value| value.into())
    }

    pub fn sub_map<S>(&self, bounds: Rect<i64>, mut f: impl FnMut(&T) -> S) -> Field<S> {
        assert!(self.bounds.contains_rect(bounds));
        Field::from_map(bounds, |index| f(&self[index]))
    }

    pub fn into_vec(self) -> Vec<T> {
        self.elems
    }
}

impl<T: Clone> Field<T> {
    pub fn filled(bounds: Rect<i64>, value: T) -> Self {
        let size = bounds.width() as usize * bounds.height() as usize;
        Self {
            bounds,
            elems: vec![value; size],
        }
    }

    pub fn blit(&mut self, other: &Self) {
        for (index, value) in other.enumerate() {
            self.set(index, value.clone());
        }
    }

    pub fn sub(&self, bounds: Rect<i64>) -> Self {
        self.sub_map(bounds, |value| value.clone())
    }

    pub fn clipped(&self, bounds: Rect<i64>) -> Self {
        self.sub(bounds.intersect(self.bounds))
    }

    /// Scales by the given scale. The bounds rectangle is also simply multiplied by the scale.
    pub fn integer_upscale(&self, scale: i64) -> Field<T> {
        assert!(scale > 0);
        Self::from_map(self.bounds * scale, |pixel| {
            self[pixel.div_euclid(scale)].clone()
        })
    }
}

impl RgbaField {
    const PNG_URL_HEADER: &'static str = "data:image/png;base64,";

    fn from_imageio_bitmap(imageio_bitmap: &image::RgbaImage) -> Self {
        let mut bitmap = Self::filled(
            Rect::low_size(
                Point::ZERO,
                Point(
                    imageio_bitmap.width() as i64,
                    imageio_bitmap.height() as i64,
                ),
            ),
            Rgba8::ZERO,
        );

        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let image::Rgba(rgba) = imageio_bitmap.get_pixel(x as u32, y as u32);
                bitmap[(x, y)] = Rgba8::from(*rgba);
            }
        }

        bitmap
    }

    pub fn load_from_memory(memory: &[u8]) -> Result<Self, image::ImageError> {
        let imageio_bitmap = image::load_from_memory(memory)?.into_rgba8();
        Ok(Self::from_imageio_bitmap(&imageio_bitmap))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, image::ImageError> {
        let imageio_bitmap = image::open(path)?.into_rgba8();
        Ok(Self::from_imageio_bitmap(&imageio_bitmap))
    }

    pub fn to_imageio(&self) -> image::RgbaImage {
        let mut imageio_bitmap = image::RgbaImage::new(self.width() as u32, self.height() as u32);

        for index in self.indices() {
            let imageio_rgba = image::Rgba(self[index].to_array());
            let offset = (index - self.bounds.low()).cwise_as::<u32>();
            imageio_bitmap.put_pixel(offset.x, offset.y, imageio_rgba);
        }

        imageio_bitmap
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.to_imageio().save(path)?;
        Ok(())
    }

    /// Uses more aggressive compression than save()
    pub fn save_png(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        use image::{
            ExtendedColorType,
            codecs::png::{CompressionType, FilterType, PngEncoder},
        };

        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);

        // Create PngEncoder with best compression
        let encoder =
            PngEncoder::new_with_quality(writer, CompressionType::Best, FilterType::Adaptive);

        // Encode the image buffer
        let bytes = self.as_raw();
        encoder.write_image(
            bytes,
            self.width() as u32,
            self.height() as u32,
            ExtendedColorType::Rgba8,
        )?;

        Ok(())
    }

    pub fn to_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        self.to_imageio()
            .write_to(&mut cursor, image::ImageFormat::Png)?;
        Ok(cursor.into_inner())
    }

    pub fn is_plain(&self, color: Rgba8) -> bool {
        self.iter().all(|pixel| pixel == &color)
    }

    pub fn is_zero(&self) -> bool {
        self.is_plain(Rgba8::TRANSPARENT)
    }

    pub fn as_raw(&self) -> &[u8] {
        bytemuck::cast_slice(self.as_slice())
    }

    pub fn into_material(self) -> MaterialField {
        self.into_map(|rgba| rgba.into())
    }

    pub fn decode_base64_png(encoded: &str) -> anyhow::Result<Self> {
        // Strip DATA_URL_HEADER
        if encoded.len() <= Self::PNG_URL_HEADER.len() {
            anyhow::bail!("Encoded string too short");
        }
        let payload = &encoded[Self::PNG_URL_HEADER.len()..];
        let png = BASE64.decode(payload.as_bytes())?;
        let rgba_field = RgbaField::load_from_memory(&png)?;
        Ok(rgba_field)
    }

    pub fn encode_base64_png(&self) -> String {
        let png = self.to_png().unwrap();
        let base64_png = BASE64.encode(&png);
        format!("{}{}", Self::PNG_URL_HEADER, base64_png)
    }
}

impl MaterialField {
    pub fn into_rgba(self) -> RgbaField {
        self.into_map(|material| material.to_rgba())
    }
}

pub trait FieldIndex: Copy {
    fn x(&self) -> i64;
    fn y(&self) -> i64;

    fn point(self) -> Point<i64> {
        Point::new(self.x(), self.y())
    }
}

impl<Idx: FieldIndex, T> Index<Idx> for Field<T> {
    type Output = T;

    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<Idx: FieldIndex, T> IndexMut<Idx> for Field<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl FieldIndex for Point<i64> {
    fn x(&self) -> i64 {
        self.x
    }

    fn y(&self) -> i64 {
        self.y
    }
}

impl FieldIndex for [i64; 2] {
    fn x(&self) -> i64 {
        self[0]
    }

    fn y(&self) -> i64 {
        self[1]
    }
}

impl FieldIndex for (i64, i64) {
    fn x(&self) -> i64 {
        self.0
    }

    fn y(&self) -> i64 {
        self.1
    }
}
