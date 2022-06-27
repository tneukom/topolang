use num_traits::Inv;
use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Neg, Sub},
};

pub trait FromLossy<Value>: Sized {
    fn from_lossy(value: Value) -> Self;
}

pub trait IntoLossy<Value>: Sized {
    fn into_lossy(self) -> Value;
}

impl<Value, This> IntoLossy<Value> for This
where
    Value: FromLossy<This>,
{
    fn into_lossy(self) -> Value {
        Value::from_lossy(self)
    }
}

macro_rules! impl_from_lossy_primitive_as {
    ($t_self: ty, $t_value: ty) => {
        impl FromLossy<$t_value> for $t_self {
            fn from_lossy(value: $t_value) -> Self {
                value as Self
            }
        }
    };
}

impl_from_lossy_primitive_as!(f32, i64);
impl_from_lossy_primitive_as!(f64, i64);
impl_from_lossy_primitive_as!(f32, u64);
impl_from_lossy_primitive_as!(f64, u64);

impl_from_lossy_primitive_as!(f32, f64);
impl_from_lossy_primitive_as!(i64, f64);
impl_from_lossy_primitive_as!(u64, f64);
impl_from_lossy_primitive_as!(usize, f64);

impl_from_lossy_primitive_as!(f32, f32);
impl_from_lossy_primitive_as!(i64, f32);
impl_from_lossy_primitive_as!(u64, f32);
impl_from_lossy_primitive_as!(usize, f32);

impl_from_lossy_primitive_as!(f32, usize);
impl_from_lossy_primitive_as!(f64, usize);
impl_from_lossy_primitive_as!(i64, usize);
impl_from_lossy_primitive_as!(u64, usize);

pub trait ConstZero {
    const ZERO: Self;
}

macro_rules! impl_const_zero {
    ($t: ty, $expr: expr) => {
        impl ConstZero for $t {
            const ZERO: Self = $expr;
        }
    };
}

impl_const_zero!(f64, 0.0);
impl_const_zero!(f32, 0.0);
impl_const_zero!(i128, 0);
impl_const_zero!(i64, 0);
impl_const_zero!(i32, 0);
impl_const_zero!(i16, 0);
impl_const_zero!(i8, 0);
impl_const_zero!(u128, 0);
impl_const_zero!(u64, 0);
impl_const_zero!(u32, 0);
impl_const_zero!(u16, 0);
impl_const_zero!(u8, 0);
impl_const_zero!(isize, 0);
impl_const_zero!(usize, 0);

pub trait ConstOne {
    const ONE: Self;
}

macro_rules! impl_const_one {
    ($t: ty, $expr: expr) => {
        impl ConstOne for $t {
            const ONE: Self = $expr;
        }
    };
}

impl_const_one!(f64, 1.0);
impl_const_one!(f32, 1.0);
impl_const_one!(i128, 1);
impl_const_one!(i64, 1);
impl_const_one!(i32, 1);
impl_const_one!(i16, 1);
impl_const_one!(i8, 1);
impl_const_one!(u128, 1);
impl_const_one!(u64, 1);
impl_const_one!(u32, 1);
impl_const_one!(u16, 1);
impl_const_one!(u8, 1);
impl_const_one!(isize, 1);
impl_const_one!(usize, 1);

pub trait ConstTwo {
    const TWO: Self;
}

macro_rules! impl_const_two {
    ($t: ty, $expr: expr) => {
        impl ConstTwo for $t {
            const TWO: Self = $expr;
        }
    };
}

impl_const_two!(f64, 2.0);
impl_const_two!(f32, 2.0);
impl_const_two!(i128, 2);
impl_const_two!(i64, 2);
impl_const_two!(i32, 2);
impl_const_two!(i16, 2);
impl_const_two!(i8, 2);
impl_const_two!(u128, 2);
impl_const_two!(u64, 2);
impl_const_two!(u32, 2);
impl_const_two!(u16, 2);
impl_const_two!(u8, 2);
impl_const_two!(isize, 2);
impl_const_two!(usize, 2);

pub trait ConstNegOne {
    const NEG_ONE: Self;
}

macro_rules! impl_const_neg_one {
    ($t: ty, $expr: expr) => {
        impl ConstNegOne for $t {
            const NEG_ONE: Self = $expr;
        }
    };
}

impl_const_neg_one!(f64, -1.0);
impl_const_neg_one!(f32, -1.0);
impl_const_neg_one!(i128, -1);
impl_const_neg_one!(i64, -1);
impl_const_neg_one!(i32, -1);
impl_const_neg_one!(i16, -1);
impl_const_neg_one!(i8, -1);
impl_const_neg_one!(isize, -1);

pub trait CwiseAdd<Rhs> {
    type Output;

    fn cwise_add(self, rhs: Rhs) -> Self::Output;
}

pub trait CwiseSub<Rhs> {
    type Output;

    fn cwise_sub(self, rhs: Rhs) -> Self::Output;
}

pub trait CwiseMul<Rhs> {
    type Output;

    fn cwise_mul(self, rhs: Rhs) -> Self::Output;
}

pub trait CwiseDiv<Rhs> {
    type Output;

    fn cwise_div(self, rhs: Rhs) -> Self::Output;
}

pub trait CwiseInv {
    type Output;

    fn cwise_inv(self) -> Self::Output;
}

pub trait CwiseEuclidDivRem<Rhs> {
    type Output;

    fn cwise_euclid_div(self, rhs: Rhs) -> Self::Output;

    fn cwise_euclid_rem(self, rhs: Rhs) -> Self::Output;
}

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

pub trait EuclidDivRem<Rhs = Self> {
    type Output;

    fn euclid_div(self, rhs: Rhs) -> Self::Output;

    /// Always positive (https://en.wikipedia.org/wiki/Modulo)
    fn euclid_rem(self, rhs: Rhs) -> Self::Output;
}

macro_rules! impl_euclid_div_rem_forward_primitive {
    ($t: ty) => {
        impl EuclidDivRem<$t> for $t {
            type Output = $t;

            fn euclid_div(self, rhs: $t) -> Self::Output {
                self.div_euclid(rhs)
            }

            fn euclid_rem(self, rhs: $t) -> Self::Output {
                self.rem_euclid(rhs)
            }
        }
    };
}

impl_euclid_div_rem_forward_primitive!(u8);
impl_euclid_div_rem_forward_primitive!(u16);
impl_euclid_div_rem_forward_primitive!(u32);
impl_euclid_div_rem_forward_primitive!(u64);
impl_euclid_div_rem_forward_primitive!(u128);
impl_euclid_div_rem_forward_primitive!(usize);
impl_euclid_div_rem_forward_primitive!(i8);
impl_euclid_div_rem_forward_primitive!(i16);
impl_euclid_div_rem_forward_primitive!(i32);
impl_euclid_div_rem_forward_primitive!(i64);
impl_euclid_div_rem_forward_primitive!(i128);
impl_euclid_div_rem_forward_primitive!(isize);
impl_euclid_div_rem_forward_primitive!(f32);
impl_euclid_div_rem_forward_primitive!(f64);

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
    + ConstOne
    + ConstZero
    + ConstTwo
    + Abs
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + EuclidDivRem<Output = Self>
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

pub trait SignedNum: Num + Neg<Output = Self> + ConstNegOne {}

impl SignedNum for f64 {}

impl SignedNum for f32 {}

impl SignedNum for i8 {}

impl SignedNum for i16 {}

impl SignedNum for i32 {}

impl SignedNum for i64 {}

impl SignedNum for isize {}

pub trait FieldNum: SignedNum + Div<Output = Self> + Inv<Output = Self> + FromLossy<i64> {}

impl FieldNum for f64 {}

impl FieldNum for f32 {}
