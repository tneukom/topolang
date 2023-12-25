use bytemuck::{Pod, Zeroable};
use std::fmt::{self, Debug};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Zeroable, Pod, PartialOrd, Ord)]
#[repr(C)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Generally considered to be in sRGB color space
impl Rgba8 {
    // fn from_rgbaf(r: f)

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
        Rgba8 { r, g, b, a }
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// https://en.wikipedia.org/wiki/SRGB
    fn component_srgb_to_linear(c_u8: u8) -> f32 {
        let c = (c_u8 as f32) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_linear(self) -> [f32; 4] {
        [
            Self::component_srgb_to_linear(self.r),
            Self::component_srgb_to_linear(self.g),
            Self::component_srgb_to_linear(self.b),
            self.a as f32 / 255.0,
        ]
    }

    // Parse hex rgba color with # at first position.
    pub fn from_hex(hex: &str) -> Option<Rgba8> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some(Rgba8 { r, g, b, a })
    }

    pub const BLACK: Self = rgba8(0x00, 0x00, 0x00, 0xFF);
    pub const WHITE: Self = rgba8(0xFF, 0xFF, 0xFF, 0xFF);
    pub const RED: Self = rgba8(0xFF, 0x00, 0x00, 0xFF);
    pub const BLUE: Self = rgba8(0x00, 0x00, 0xFF, 0xFF);
    pub const GREEN: Self = rgba8(0x00, 0xFF, 0x00, 0xFF);
    pub const CYAN: Self = rgba8(0x00, 0xFF, 0xFF, 0xFF);
    pub const YELLOW: Self = rgba8(0xFF, 0xFF, 0x00, 0xFF);
    pub const MAGENTA: Self = rgba8(0xFF, 0x00, 0xFF, 0xFF);
    pub const PURPLE: Self = rgba8(0x80, 0x00, 0x80, 0xFF);
    pub const ORANGE: Self = rgba8(0xFF, 0xA5, 0x00, 0xFF);
    pub const TEAL: Self = rgba8(0x00, 0x80, 0x80, 0xFF);
    pub const TRANSPARENT: Self = rgba8(0x00, 0x00, 0x00, 0x00);
    pub const ZERO: Self = Self::TRANSPARENT;

    /// Hex 360c29
    pub const VOID: Rgba8 = Rgba8::new(0x36, 0x0C, 0x29, 0xFF);
}

pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub const fn rgb8(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

impl From<[u8; 4]> for Rgba8 {
    fn from(rgba8: [u8; 4]) -> Self {
        Rgba8 {
            r: rgba8[0],
            g: rgba8[1],
            b: rgba8[2],
            a: rgba8[3],
        }
    }
}

impl From<&[f64; 4]> for Rgba8 {
    fn from(rgbaf: &[f64; 4]) -> Self {
        Rgba8 {
            r: (rgbaf[0] * 255.0).round() as u8,
            g: (rgbaf[1] * 255.0).round() as u8,
            b: (rgbaf[2] * 255.0).round() as u8,
            a: (rgbaf[3] * 255.0).round() as u8,
        }
    }
}

impl From<&[f32; 4]> for Rgba8 {
    fn from(rgbaf: &[f32; 4]) -> Self {
        Rgba8 {
            r: (rgbaf[0] * 255.0).round() as u8,
            g: (rgbaf[1] * 255.0).round() as u8,
            b: (rgbaf[2] * 255.0).round() as u8,
            a: (rgbaf[3] * 255.0).round() as u8,
        }
    }
}

impl fmt::Display for Rgba8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

impl Debug for Rgba8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
