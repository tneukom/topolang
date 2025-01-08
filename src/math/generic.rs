use num_traits::{ConstOne, ConstZero, Inv};
use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};

pub trait Abs {
    fn abs(self) -> Self;
}

macro_rules! impl_abs_forward_primitive {
    ($t: ty) => {
        impl Abs for $t {
            fn abs(self) -> Self {
                <$t>::abs(self)
            }
        }
    };
}

macro_rules! impl_abs_noop {
    ($t: ty) => {
        impl Abs for $t {
            fn abs(self) -> Self {
                self
            }
        }
    };
}

impl_abs_forward_primitive!(f64);
impl_abs_forward_primitive!(f32);
impl_abs_forward_primitive!(i8);
impl_abs_forward_primitive!(i16);
impl_abs_forward_primitive!(i32);
impl_abs_forward_primitive!(i64);
impl_abs_forward_primitive!(i128);
impl_abs_forward_primitive!(isize);
impl_abs_noop!(u8);
impl_abs_noop!(u16);
impl_abs_noop!(u32);
impl_abs_noop!(u64);
impl_abs_noop!(u128);
impl_abs_noop!(usize);

pub trait Dot<Rhs = Self> {
    type Output;

    fn dot(self, rhs: Rhs) -> Self::Output;
}

pub trait MinMax {
    fn min(self, rhs: Self) -> Self;
    fn max(self, rhs: Self) -> Self;
}

macro_rules! impl_min_max_ord {
    ($t: ty) => {
        impl MinMax for $t {
            fn min(self, rhs: Self) -> Self {
                std::cmp::min(self, rhs)
            }

            fn max(self, rhs: Self) -> Self {
                std::cmp::max(self, rhs)
            }
        }
    };
}

impl_min_max_ord!(u8);
impl_min_max_ord!(u16);
impl_min_max_ord!(u32);
impl_min_max_ord!(u64);
impl_min_max_ord!(u128);
impl_min_max_ord!(usize);
impl_min_max_ord!(i8);
impl_min_max_ord!(i16);
impl_min_max_ord!(i32);
impl_min_max_ord!(i64);
impl_min_max_ord!(i128);
impl_min_max_ord!(isize);

macro_rules! impl_min_max_primitive {
    ($t: ty) => {
        impl MinMax for $t {
            fn min(self, rhs: Self) -> Self {
                <$t>::min(self, rhs)
            }

            fn max(self, rhs: Self) -> Self {
                <$t>::max(self, rhs)
            }
        }
    };
}

impl_min_max_primitive!(f32);
impl_min_max_primitive!(f64);

pub trait Num:
    Copy
    + Sized
    + Debug
    + PartialOrd
    + Abs
    + ConstZero
    + ConstOne
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + MinMax
{
}

impl Num for u8 {}

impl Num for i8 {}

impl Num for u16 {}

impl Num for i16 {}

impl Num for u32 {}

impl Num for i32 {}

impl Num for u64 {}

impl Num for i64 {}

impl Num for u128 {}

impl Num for i128 {}

impl Num for usize {}

impl Num for isize {}

impl Num for f64 {}

impl Num for f32 {}

pub trait SignedNum: Num + Neg<Output = Self> {}

impl SignedNum for f64 {}

impl SignedNum for f32 {}

impl SignedNum for i8 {}

impl SignedNum for i16 {}

impl SignedNum for i32 {}

impl SignedNum for i64 {}

impl SignedNum for isize {}

pub trait FieldNum: SignedNum + Div<Output = Self> + Inv<Output = Self> {}

impl FieldNum for f64 {}

impl FieldNum for f32 {}
