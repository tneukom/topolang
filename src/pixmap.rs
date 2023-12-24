use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, rgba8::Rgba8},
};
use std::{collections::BTreeMap, path::Path};

pub fn pixmap_from_bitmap(bitmap: &Bitmap) -> BTreeMap<Pixel, Rgba8> {
    let mut dict: BTreeMap<Pixel, Rgba8> = BTreeMap::new();
    for idx in bitmap.indices() {
        let color = bitmap[idx];

        if color.a == 0 && color != Rgba8::TRANSPARENT {
            println!("Bitmap should not contain colors with alpha = 0 but rgb != 0")
        }

        let pixel: Pixel = idx.cwise_try_into::<i64>().unwrap().into();
        dict.insert(pixel, color);
    }
    dict
}

pub fn pixmap_from_path(path: impl AsRef<Path>) -> anyhow::Result<BTreeMap<Pixel, Rgba8>> {
    let bitmap = Bitmap::from_path(path)?;
    Ok(pixmap_from_bitmap(&bitmap))
}

pub fn pixmap_remove_color(pixmap: &mut BTreeMap<Pixel, Rgba8>, color: Rgba8) {
    pixmap.retain(|_, &mut pixel_color| pixel_color != color);
}
