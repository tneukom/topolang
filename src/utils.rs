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

pub trait KeyValueItertools: Iterator {
    /// Filter by the key (fist item in pair) but return the value (second item in pair)
    fn filter_key_by_value<K, V>(self, pred: impl FnMut(&V) -> bool) -> impl Iterator<Item = K>
    where
        Self: Iterator<Item = (K, V)>;

    fn find_key_by_value<K, V>(self, pred: impl FnMut(&V) -> bool) -> Option<K>
    where
        Self: Iterator<Item = (K, V)>;

    fn filter_by_value<K, V>(self, pred: impl FnMut(&V) -> bool) -> impl Iterator<Item = (K, V)>
    where
        Self: Iterator<Item = (K, V)>;
}

impl<Iter: Iterator> KeyValueItertools for Iter {
    fn filter_key_by_value<K, V>(self, mut pred: impl FnMut(&V) -> bool) -> impl Iterator<Item = K>
    where
        Self: Iterator<Item = (K, V)>,
    {
        self.filter_map(move |(key, value)| pred(&value).then_some(key))
    }

    fn find_key_by_value<K, V>(mut self, mut pred: impl FnMut(&V) -> bool) -> Option<K>
    where
        Self: Iterator<Item = (K, V)>,
    {
        self.find_map(|(key, value)| pred(&value).then_some(key))
    }

    fn filter_by_value<K, V>(self, mut pred: impl FnMut(&V) -> bool) -> impl Iterator<Item = (K, V)>
    where
        Self: Iterator<Item = (K, V)>,
    {
        self.filter_map(move |(key, value)| pred(&value).then_some((key, value)))
    }
}

