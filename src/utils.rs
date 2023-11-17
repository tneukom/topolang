use itertools::Itertools;
use std::{array, fmt::Debug, hash::Hash, rc::Rc};

pub struct ById<T>(pub T);

impl<T> Hash for ById<Rc<T>> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // hash by raw pointer
        Rc::as_ptr(&self.0).hash(state);
    }
}

impl<T> PartialEq for ById<Rc<T>> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for ById<Rc<T>> {}

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
