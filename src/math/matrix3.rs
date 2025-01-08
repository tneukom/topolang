use crate::math::{affine_map::AffineMap, generic::Num};
use num_traits::AsPrimitive;

#[derive(Debug, Clone, Copy)]
pub struct Matrix3<T> {
    pub a11: T,
    pub a12: T,
    pub a13: T,
    pub a21: T,
    pub a22: T,
    pub a23: T,
    pub a31: T,
    pub a32: T,
    pub a33: T,
}

impl<T> Matrix3<T> {
    pub const fn new(
        a11: T,
        a12: T,
        a13: T,
        a21: T,
        a22: T,
        a23: T,
        a31: T,
        a32: T,
        a33: T,
    ) -> Self {
        Self {
            a11,
            a12,
            a13,
            a21,
            a22,
            a23,
            a31,
            a32,
            a33,
        }
    }

    pub fn transpose(self) -> Self {
        Self::new(
            self.a11, self.a21, self.a31, self.a12, self.a22, self.a32, self.a13, self.a23,
            self.a33,
        )
    }

    pub fn cwise_as<S>(self) -> Matrix3<S>
    where
        T: AsPrimitive<S>,
        S: Copy + 'static,
    {
        Matrix3::new(
            self.a11.as_(),
            self.a12.as_(),
            self.a13.as_(),
            self.a21.as_(),
            self.a22.as_(),
            self.a23.as_(),
            self.a31.as_(),
            self.a32.as_(),
            self.a33.as_(),
        )
    }
}

impl<T: Copy> Matrix3<T> {
    pub const fn constant(c: T) -> Self {
        Self::new(c, c, c, c, c, c, c, c, c)
    }
}

impl<T: Num> Matrix3<T> {
    pub const fn diagonal(a11: T, a22: T, a33: T) -> Self {
        Self::new(
            a11,
            T::ZERO,
            T::ZERO,
            T::ZERO,
            a22,
            T::ZERO,
            T::ZERO,
            T::ZERO,
            a33,
        )
    }

    pub const ZERO: Self = Self::constant(T::ZERO);
    pub const ID: Self = Self::diagonal(T::ONE, T::ONE, T::ONE);

    pub const fn to_array(self) -> [T; 9] {
        [
            self.a11, self.a12, self.a13, self.a21, self.a22, self.a23, self.a31, self.a32,
            self.a33,
        ]
    }
}

impl<T: Num> From<AffineMap<T>> for Matrix3<T> {
    fn from(phi: AffineMap<T>) -> Self {
        let mut result = Self::ZERO;

        result.a11 = phi.linear.a11;
        result.a12 = phi.linear.a12;
        result.a21 = phi.linear.a21;
        result.a22 = phi.linear.a22;

        result.a13 = phi.constant.x;
        result.a23 = phi.constant.y;
        result.a33 = T::ONE;

        result
    }
}
