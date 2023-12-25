use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, rgba8::Rgba8},
    topology::Border,
};
use std::{collections::BTreeMap, ops::Index, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pixmap {
    pub map: BTreeMap<Pixel, Rgba8>,
}

impl Pixmap {
    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        let mut map: BTreeMap<Pixel, Rgba8> = BTreeMap::new();
        for idx in bitmap.indices() {
            let color = bitmap[idx];

            if color.a == 0 && color != Rgba8::TRANSPARENT {
                println!("Bitmap should not contain colors with alpha = 0 but rgb != 0")
            }

            let pixel: Pixel = idx.cwise_try_into::<i64>().unwrap().into();
            map.insert(pixel, color);
        }
        Self { map }
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = Bitmap::from_path(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }

    pub fn without_color(mut self, color: Rgba8) -> Self {
        self.map.retain(|_, &mut pixel_color| pixel_color != color);
        self
    }

    pub fn get(&self, pixel: &Pixel) -> Option<&Rgba8> {
        self.map.get(pixel)
    }

    pub fn remove(&mut self, pixel: &Pixel) -> Option<Rgba8> {
        self.map.remove(pixel)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Pixel> + Clone {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Rgba8> + Clone {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn extract_right(&mut self, boundary: &Border) -> Pixmap {
        // Extract pixels left of inner_border
        let mut right = BTreeMap::new();
        for pixel in boundary.right_pixels() {
            let color = self.remove(&pixel).unwrap();
            right.insert(pixel, color);
        }

        Self { map: right }
    }
}

impl Index<&Pixel> for Pixmap {
    type Output = Rgba8;

    fn index(&self, pixel: &Pixel) -> &Self::Output {
        &self.map[pixel]
    }
}
