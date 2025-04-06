use crate::{field::RgbaField, math::rgba8::Rgba8};
use itertools::Itertools;
use std::sync::OnceLock;

pub struct Palette {
    /// Non-empty set of colors
    pub colors: Vec<Rgba8>,

    pub name: String,
    pub link: String,
}

impl Palette {
    pub fn from_bitmap(
        bitmap: &RgbaField,
        name: impl Into<String>,
        link: impl Into<String>,
    ) -> Self {
        let colors: Vec<_> = bitmap.iter().copied().unique().collect();
        assert!(!colors.is_empty());

        Self {
            colors,
            name: name.into(),
            link: link.into(),
        }
    }

    pub fn palette_pico8() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("pico8.png")).unwrap();
        Self::from_bitmap(&bitmap, "PICO-8", "https://lospec.com/palette-list/pico-8")
    }

    pub fn palette_desatur8() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("desatur8.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "DESATUR8",
            "https://lospec.com/palette-list/desatur8",
        )
    }

    pub fn palette_windows16() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("windows16.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "WIN16",
            "https://lospec.com/palette-list/microsoft-vga",
        )
    }

    pub fn palette_na16() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("na16.png")).unwrap();
        Self::from_bitmap(&bitmap, "NA16", "https://lospec.com/palette-list/na16")
    }

    pub fn palette_hept32() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("hept32.png")).unwrap();
        Self::from_bitmap(&bitmap, "HEPT32", "https://lospec.com/palette-list/hept32")
    }

    pub fn palette_superfuture25() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("superfuture25.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "FUTURE25",
            "https://lospec.com/palette-list/superfuture25",
        )
    }

    pub fn palette_distinct20() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("distinct20.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "DISTINCT",
            "https://sashamaps.net/docs/resources/20-colors/",
        )
    }

    pub fn palette_pxls_space() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("pxls_space.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "PXLS.SPACE",
            "https://lospec.com/palette-list/pxlsspace-nov2023",
        )
    }

    pub fn palette_r_place() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("r_place_32.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "R/PLACE",
            "https://lospec.com/palette-list/r-place-2022-32-colors",
        )
    }

    pub fn palettes() -> &'static [Palette] {
        static PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();
        let palettes = PALETTES.get_or_init(|| {
            vec![
                Palette::palette_r_place(),
                Palette::palette_pico8(),
                Palette::palette_windows16(),
                // Palette::palette_na16(),
                // Palette::palette_desatur8(),
                // Palette::palette_hept32(),
                // Palette::palette_superfuture25(),
                Palette::palette_distinct20(),
                // Palette::palette_pxls_space(),
            ]
        });
        palettes.as_slice()
    }
}
