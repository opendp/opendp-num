use crate::{Add, Backend, ExactBinary, ExactUnary, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default)]
pub struct Malachite;

impl Backend for Malachite {
    type Natural = malachite::Natural;
    type Integer = malachite::Integer;
    type Rational = malachite::Rational;
}

macro_rules! exact_binary {
    ($ty:ty; $($op:ty, $trait:ident, $method:ident);+ $(;)?) => {
        $(impl ExactBinary<$op, $ty> for Malachite {
            #[inline]
            fn eval(lhs: &$ty, rhs: &$ty) -> $ty {
                core::ops::$trait::$method(lhs, rhs)
            }
        })+
    };
}

exact_binary!(malachite::Natural; Add, Add, add; Mul, Mul, mul);
exact_binary!(malachite::Integer; Add, Add, add; Sub, Sub, sub; Mul, Mul, mul);
exact_binary!(malachite::Rational; Add, Add, add; Sub, Sub, sub; Mul, Mul, mul);

impl ExactUnary<Neg, malachite::Integer> for Malachite {
    fn eval(value: &malachite::Integer) -> malachite::Integer {
        -value
    }
}
impl ExactUnary<Neg, malachite::Rational> for Malachite {
    fn eval(value: &malachite::Rational) -> malachite::Rational {
        -value
    }
}
