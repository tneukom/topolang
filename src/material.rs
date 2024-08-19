use crate::math::rgba8::Rgba8;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, Copy)]
pub struct Material {
    color: Rgba8,
}

impl Material {
    pub const RIGID_ALPHA: u8 = 170;
    pub const NORMAL_ALPHA: u8 = 255;

    pub const VOID: Self = Self::new(Rgba8::VOID);
    pub const RULE_FRAME: Self = Self::VOID;
    pub const TRANSPARENT: Self = Self::new(Rgba8::TRANSPARENT);
    pub const BLACK: Self = Self::new(Rgba8::BLACK);
    // const RULE_ARROW:

    pub const fn color(self) -> Rgba8 {
        self.color
    }

    pub const fn new(rgba: Rgba8) -> Self {
        Self { color: rgba }
    }

    pub fn is_rigid(self) -> bool {
        self.color.a == Self::RIGID_ALPHA
    }

    pub fn rigid(mut self) -> Material {
        assert!(self.is_normal());
        self.color.a = Self::RIGID_ALPHA;
        self
    }

    pub fn is_normal(self) -> bool {
        self.color.a == Self::NORMAL_ALPHA
    }
}

impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        // For rule frame and arrow (alpha = 180, 181) all rgb values are considered equal
        if self.color.a == 180 || self.color.a == 181 {
            self.color.a == other.color.a
        } else {
            self.color == other.color
        }
    }
}

impl Eq for Material {}

impl Ord for Material {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.color.a == 180 || self.color.a == 181 {
            self.color.a.cmp(&other.color.a)
        } else {
            self.color.cmp(&other.color)
        }
    }
}

impl PartialOrd for Material {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Material {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.color.a == 180 || self.color.a == 181 {
            self.color.a.hash(state);
        } else {
            self.color.hash(state);
        }
    }
}

impl From<Rgba8> for Material {
    fn from(rgba: Rgba8) -> Self {
        Self::from(&rgba)
    }
}

impl From<&Rgba8> for Material {
    fn from(rgba: &Rgba8) -> Self {
        Material { color: *rgba }
    }
}

impl From<Material> for Rgba8 {
    fn from(material: Material) -> Self {
        material.color
    }
}
