use itertools::Itertools;
use std::{array, collections::BTreeSet, fmt::Debug, rc::Rc};

#[derive(Debug, PartialEq, Eq)]
pub enum UniqueError {
    Missing,
    NotUnique,
}

pub trait IteratorExt: Iterator + Sized {
    fn unique_item(self) -> Result<Self::Item, UniqueError> {
        match self.at_most_one() {
            Ok(None) => Err(UniqueError::Missing),
            Ok(Some(some)) => Ok(some),
            Err(_) => Err(UniqueError::NotUnique),
        }
    }

    // fn indices(self) -> impl Iterator<Item = Self::Item> {
    //     self.enumerate().map(|(i, _)| i)
    // }
}

impl<T: Sized> IteratorExt for T where T: Iterator {}

#[cfg(test)]
mod test {
    use crate::utils::{IteratorExt, UniqueError};

    #[test]
    fn iterator_unique_item() {
        assert_eq!(
            [1, 2].iter().unique_item().unwrap_err(),
            UniqueError::NotUnique
        );
        let ls: [i64; 0] = [];
        assert_eq!(ls.iter().unique_item().unwrap_err(), UniqueError::Missing);
    }
}

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

// TODO: Remove once https://github.com/rust-lang/rust/issues/93610 is merged
pub trait RcExt<T> {
    fn unwrap_or_clone2(this: Self) -> T;
}

impl<T> RcExt<T> for Rc<T>
where
    T: Clone,
{
    fn unwrap_or_clone2(this: Self) -> T {
        Rc::try_unwrap(this).unwrap_or_else(|rc| (*rc).clone())
    }
}

// TODO: Use each_ref when stable
// https://www.reddit.com/r/learnrust/comments/10jo2kj/how_do_you_convert_an_array_into_an_array_of/
// https://doc.rust-lang.org/std/primitive.array.html#method.each_ref
pub fn array_map_ref<T, const N: usize, F, U>(array: &[T; N], mut f: F) -> [U; N]
where
    F: FnMut(&T) -> U,
{
    array::from_fn(|i| f(&array[i]))
}

pub fn array_take_nth<T, const N: usize>(array: [T; N], n: usize) -> T {
    // nth uses advance_by() of the array iterator, should be fast
    // https://doc.rust-lang.org/src/core/array/iter.rs.html
    array.into_iter().nth(n).unwrap()
}

pub fn all_equal<T: Eq>(mut iter: impl Iterator<Item = T>) -> bool {
    match iter.next() {
        None => true,
        Some(first) => iter.all(|el| el == first),
    }
}

/// O(n^2) use for small lists only
pub fn find_duplicate_by<T>(ls: &Vec<T>, eq: impl Fn(&T, &T) -> bool) -> Option<(usize, usize)> {
    for i in 0..ls.len() {
        for j in 0..i {
            if eq(&ls[i], &ls[j]) {
                return Some((i, j));
            }
        }
    }

    None
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

pub trait IteratorPlus<Item>: Iterator<Item = Item> + IntoIterator<Item = Item> + Clone {}

impl<I, Item> IteratorPlus<Item> for I where
    I: Iterator<Item = Item> + IntoIterator<Item = Item> + Clone
{
}
