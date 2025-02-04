use std::{collections::BTreeSet, fmt::Debug};

pub trait ReflectEnum: Sized + Copy + 'static {
    fn all() -> &'static [Self];

    fn as_str(self) -> &'static str;

    fn from_str(str: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|&choice| choice.as_str() == str)
            .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UndirectedEdge<T> {
    pub a: T,
    pub b: T,
}

impl<T: Ord> UndirectedEdge<T> {
    pub fn new(a: T, b: T) -> Self {
        if a < b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

pub type UndirectedGraph<T> = BTreeSet<UndirectedEdge<T>>;

pub trait IntoT: Sized {
    fn intot<S: From<Self>>(self) -> S;
}

impl<T> IntoT for T {
    fn intot<S: From<Self>>(self) -> S {
        S::from(self)
    }
}
