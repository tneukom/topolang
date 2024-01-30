use crate::{bitmap::Bitmap, math::rgba8::Rgba8};
use itertools::Itertools;

pub struct Palette {
    /// Non-empty set of colors
    pub colors: Vec<Rgba8>,

    pub name: String,
    pub link: String,
}

impl Palette {
    pub fn from_bitmap(bitmap: &Bitmap, name: impl Into<String>, link: impl Into<String>) -> Self {
        let colors: Vec<_> = bitmap.iter().copied().unique().collect();
        assert!(!colors.is_empty());

        Self {
            colors,
            name: name.into(),
            link: link.into(),
        }
    }

    pub fn palette_pico8() -> Palette {
        let bitmap = Bitmap::load_from_memory(include_bytes!("pico8.png")).unwrap();
        Self::from_bitmap(&bitmap, "PICO-8", "https://lospec.com/palette-list/pico-8")
    }

    pub fn palette_desatur8() -> Palette {
        let bitmap = Bitmap::load_from_memory(include_bytes!("desatur8.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "DESATUR8",
            "https://lospec.com/palette-list/desatur8",
        )
    }

    pub fn palette_windows16() -> Palette {
        let bitmap = Bitmap::load_from_memory(include_bytes!("windows16.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "WINDOWS-16",
            "https://lospec.com/palette-list/microsoft-vga",
        )
    }

    pub fn palette_na16() -> Palette {
        let bitmap = Bitmap::load_from_memory(include_bytes!("na16.png")).unwrap();
        Self::from_bitmap(&bitmap, "NA16", "https://lospec.com/palette-list/na16")
    }
}
