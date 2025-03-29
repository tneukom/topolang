use bytemuck::{Pod, Zeroable};
use egui::Color32;
use std::{
    fmt::{self, Debug},
    ops::{Add, Mul},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Zeroable, Pod, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct Rgb<T> {
    pub r: T,
    pub g: T,
    pub b: T,
}

#[allow(non_snake_case)]
pub const fn Rgb<T>(r: T, g: T, b: T) -> Rgb<T> {
    Rgb::new(r, g, b)
}

pub type Rgb8 = Rgb<u8>;

impl<T> Rgb<T> {
    pub const fn new(r: T, g: T, b: T) -> Self {
        Self { r, g, b }
    }

    pub fn to_array(self) -> [T; 3] {
        [self.r, self.g, self.b]
    }
}

impl Rgb<u8> {
    pub fn to_f32(self) -> Rgb<f32> {
        Rgb::new(
            (self.r as f32) / 255.0,
            (self.g as f32) / 255.0,
            (self.b as f32) / 255.0,
        )
    }
}

impl Rgb<f32> {
    pub fn to_u8(self) -> Rgb<u8> {
        Rgb::new(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        )
    }
}

impl Mul<Rgb<f32>> for f32 {
    type Output = Rgb<f32>;

    fn mul(self, rhs: Rgb<f32>) -> Self::Output {
        Rgb::new(self * rhs.r, self * rhs.g, self * rhs.b)
    }
}

impl<T> Add for Rgb<T>
where
    T: Add<Output = T>,
{
    type Output = Rgb<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Rgb::new(self.r + rhs.r, self.g + rhs.g, self.b + rhs.b)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Zeroable, Pod, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct Rgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

#[allow(non_snake_case)]
pub const fn Rgba<T>(r: T, g: T, b: T, a: T) -> Rgba<T> {
    Rgba::new(r, g, b, a)
}

pub type Rgba8 = Rgba<u8>;
pub type RgbaF = Rgba<f32>;

impl<T> Rgba<T> {
    pub const fn new(r: T, g: T, b: T, a: T) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(self) -> [T; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl<T: Copy> Rgba<T> {
    pub const fn from_rgb_a(rgb: Rgb<T>, a: T) -> Self {
        Self::new(rgb.r, rgb.g, rgb.b, a)
    }

    pub const fn rgb(self) -> Rgb<T> {
        Rgb::new(self.r, self.g, self.b)
    }
}

/// Generally considered to be in sRGB color space
impl Rgba<u8> {
    pub const fn from_rgb(rgb: Rgb<u8>) -> Self {
        Self::new(rgb.r, rgb.g, rgb.b, 255)
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    pub fn srgb_to_linear(self) -> Self {
        self.to_f32().srgb_to_linear().to_u8()
    }

    pub fn linear_to_srgb(self) -> Self {
        self.to_f32().linear_to_srgb().to_u8()
    }

    pub fn to_f32(self) -> Rgba<f32> {
        Rgba::new(
            (self.r as f32) / 255.0,
            (self.g as f32) / 255.0,
            (self.b as f32) / 255.0,
            (self.a as f32) / 255.0,
        )
    }

    // Parse hex rgba color with # at first position.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some(Self { r, g, b, a })
    }

    /// Alpha blending, see Rgba<f32>::blend
    pub fn blend_rgb(self, dst: Rgb<u8>) -> Rgb<u8> {
        self.to_f32().blend_rgb(dst.to_f32()).to_u8()
    }

    pub const BLACK: Self = Rgba(0x00, 0x00, 0x00, 0xFF);
    pub const WHITE: Self = Rgba(0xFF, 0xFF, 0xFF, 0xFF);
    pub const RED: Self = Rgba(0xFF, 0x00, 0x00, 0xFF);
    pub const BLUE: Self = Rgba(0x00, 0x00, 0xFF, 0xFF);
    pub const GREEN: Self = Rgba(0x00, 0xFF, 0x00, 0xFF);
    pub const CYAN: Self = Rgba(0x00, 0xFF, 0xFF, 0xFF);
    pub const YELLOW: Self = Rgba(0xFF, 0xFF, 0x00, 0xFF);
    pub const MAGENTA: Self = Rgba(0xFF, 0x00, 0xFF, 0xFF);
    pub const PURPLE: Self = Rgba(0x80, 0x00, 0x80, 0xFF);
    pub const ORANGE: Self = Rgba(0xFF, 0xA5, 0x00, 0xFF);
    pub const TEAL: Self = Rgba(0x00, 0x80, 0x80, 0xFF);
    pub const TRANSPARENT: Self = Rgba(0x00, 0x00, 0x00, 0x00);
    pub const ZERO: Self = Self::TRANSPARENT;
}

impl Rgba<f32> {
    pub fn to_u8(self) -> Rgba<u8> {
        Rgba::new(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// https://en.wikipedia.org/wiki/SRGB
    fn component_srgb_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// https://en.wikipedia.org/wiki/SRGB
    fn component_linear_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    pub fn linear_to_srgb(self) -> Self {
        Self::new(
            Self::component_linear_to_srgb(self.r),
            Self::component_linear_to_srgb(self.g),
            Self::component_linear_to_srgb(self.b),
            self.a,
        )
    }

    pub fn srgb_to_linear(self) -> Self {
        Self::new(
            Self::component_srgb_to_linear(self.r),
            Self::component_srgb_to_linear(self.g),
            Self::component_srgb_to_linear(self.b),
            self.a,
        )
    }

    /// Alpha blending, dst alpha is assumed to be 1.
    /// See https://ciechanow.ski/alpha-compositing/ also for compositing with dst alpha != 1
    pub fn blend_rgb(self, dst: Rgb<f32>) -> Rgb<f32> {
        self.a * self.rgb() + (1.0 - self.a) * dst
    }
}

// TODO: Is this the idiomatic way of putting constants in a namespace?
pub struct Pico8Palette {}

#[allow(dead_code)]
impl Pico8Palette {
    pub const BLACK: Rgb8 = Rgb(0, 0, 0);
    pub const DARK_BLUE: Rgb8 = Rgb(29, 43, 83);
    pub const DARK_PURPLE: Rgb8 = Rgb(126, 37, 83);
    pub const DARK_GREEN: Rgb8 = Rgb(0, 135, 81);
    pub const BROWN: Rgb8 = Rgb(171, 82, 54);
    pub const DARK_GREY: Rgb8 = Rgb(95, 87, 79);
    pub const LIGHT_GREY: Rgb8 = Rgb(194, 195, 199);
    pub const WHITE: Rgb8 = Rgb(255, 241, 232);
    pub const RED: Rgb8 = Rgb(255, 0, 77);
    pub const ORANGE: Rgb8 = Rgb(255, 163, 0);
    pub const YELLOW: Rgb8 = Rgb(255, 236, 39);
    pub const GREEN: Rgb8 = Rgb(0, 228, 54);
    pub const BLUE: Rgb8 = Rgb(41, 173, 255);
    pub const LAVENDER: Rgb8 = Rgb(131, 118, 156);
    pub const PINK: Rgb8 = Rgb(255, 119, 168);
    pub const LIGHT_PEACH: Rgb8 = Rgb(255, 204, 170);
}

impl<T: Copy> From<[T; 4]> for Rgba<T> {
    fn from(rgba: [T; 4]) -> Self {
        Self::new(rgba[0], rgba[1], rgba[2], rgba[3])
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

impl From<Rgba8> for Color32 {
    fn from(value: Rgba8) -> Self {
        Color32::from_rgba_unmultiplied(value.r, value.g, value.b, value.a)
    }
}
