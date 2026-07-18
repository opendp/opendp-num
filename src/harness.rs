//! Reusable backend contract harnesses.

use crate::{Add, ExactBinary, Mul, Sub};

pub fn exact_ring_smoke<B, T>(zero: T, one: T, two: T)
where
    B: ExactBinary<Add, T> + ExactBinary<Sub, T> + ExactBinary<Mul, T>,
    T: Eq + core::fmt::Debug,
{
    assert_eq!(<B as ExactBinary<Add, T>>::eval(&one, &one), two);
    assert_eq!(<B as ExactBinary<Sub, T>>::eval(&two, &one), one);
    assert_eq!(<B as ExactBinary<Mul, T>>::eval(&two, &zero), zero);
}
