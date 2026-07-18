use rug::{
    Float, Integer, Rational,
    float::Round,
    ops::{AddAssignRound, DivAssignRound, MulAssignRound, PowAssignRound, SubAssignRound},
};

use crate::{
    Add, Backend, CheckedBinary, Convert, DirectedBinary, DirectedPowI, DirectedUnary, Direction,
    Div, Error, ErrorKind, ExactBinary, ExactUnary, Exp, ExpM1, FromParts, IntoParts, Ln, Ln1p,
    Log2, Mul, Neg, Result, Rounded, Sqrt, Sub,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Mpfr;

impl Backend for Mpfr {
    // Rug does not have a separate unsigned arbitrary-precision type. The
    // abstraction enforces nonnegativity at natural-number construction sites.
    type Natural = Integer;
    type Integer = Integer;
    type Rational = Rational;
}

macro_rules! exact_binary_rug {
    ($ty:ty; $($op:ty, $operator:tt);+ $(;)?) => {
        $(impl ExactBinary<$op, $ty> for Mpfr {
            #[inline]
            fn eval(lhs: &$ty, rhs: &$ty) -> $ty {
                <$ty>::from(lhs $operator rhs)
            }
        })+
    };
}

exact_binary_rug!(Integer; Add, +; Sub, -; Mul, *);
exact_binary_rug!(Rational; Add, +; Sub, -; Mul, *);

impl ExactUnary<Neg, Integer> for Mpfr {
    #[inline]
    fn eval(value: &Integer) -> Integer {
        Integer::from(-value)
    }
}

impl ExactUnary<Neg, Rational> for Mpfr {
    #[inline]
    fn eval(value: &Rational) -> Rational {
        Rational::from(-value)
    }
}

impl CheckedBinary<Div, Rational> for Mpfr {
    fn eval(lhs: &Rational, rhs: &Rational) -> Result<Rational> {
        if rhs.is_zero() {
            return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
        }
        Ok(Rational::from(lhs / rhs))
    }
}

impl FromParts<Rational, Integer, Integer> for Mpfr {
    fn from_parts(numerator: Integer, denominator: Integer) -> Result<Rational> {
        if denominator == 0 {
            return Err(Error::new(
                ErrorKind::DivisionByZero,
                "zero rational denominator",
            ));
        }
        if denominator < 0 {
            return Ok(Rational::from((-numerator, -denominator)));
        }
        Ok(Rational::from((numerator, denominator)))
    }
}

impl IntoParts<Rational, Integer, Integer> for Mpfr {
    #[inline]
    fn into_parts(value: Rational) -> (Integer, Integer) {
        value.into_numer_denom()
    }
}

fn to_round(direction: Direction) -> Round {
    match direction {
        Direction::Down => Round::Down,
        Direction::Nearest => Round::Nearest,
        Direction::Up => Round::Up,
    }
}

fn non_finite_input() -> Error {
    Error::new(ErrorKind::NonFinite, "input must be finite")
}

fn domain_error() -> Error {
    Error::new(ErrorKind::Domain, "input is outside the operation domain")
}

fn overflow_error() -> Error {
    Error::new(ErrorKind::Overflow, "operation produced a non-finite value")
}

trait PrimitiveFloat: Copy {
    const PRECISION: u32;

    fn is_finite(self) -> bool;
    fn from_mpfr(value: &Float, round: Round) -> Self;
}

impl PrimitiveFloat for f64 {
    const PRECISION: u32 = f64::MANTISSA_DIGITS;

    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }

    fn from_mpfr(value: &Float, round: Round) -> Self {
        value.to_f64_round(round)
    }
}

impl PrimitiveFloat for f32 {
    const PRECISION: u32 = f32::MANTISSA_DIGITS;

    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }

    fn from_mpfr(value: &Float, round: Round) -> Self {
        value.to_f32_round(round)
    }
}

fn finish<T: PrimitiveFloat>(value: Float, direction: Direction) -> Result<Rounded<T>> {
    if value.is_nan() {
        return Err(domain_error());
    }
    let output = T::from_mpfr(&value, to_round(direction));
    if !output.is_finite() {
        return Err(overflow_error());
    }
    Ok(Rounded::new(output, direction))
}

macro_rules! directed_unary {
    ($ty:ty; $op:ty, $method:ident, $domain:expr) => {
        impl DirectedUnary<$op, $ty> for Mpfr {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if !($domain)(value) {
                    return Err(domain_error());
                }
                let round = to_round(direction);
                let mut output = Float::with_val(<$ty as PrimitiveFloat>::PRECISION, value);
                let previous = output.$method(round);
                output.subnormalize_ieee_round(previous, round);
                finish(output, direction)
            }
        }
    };
}

directed_unary!(f64; Ln, ln_round, |x: f64| x > 0.0);
directed_unary!(f32; Ln, ln_round, |x: f32| x > 0.0);
directed_unary!(f64; Log2, log2_round, |x: f64| x > 0.0);
directed_unary!(f32; Log2, log2_round, |x: f32| x > 0.0);
directed_unary!(f64; Ln1p, ln_1p_round, |x: f64| x > -1.0);
directed_unary!(f32; Ln1p, ln_1p_round, |x: f32| x > -1.0);
directed_unary!(f64; Exp, exp_round, |_x: f64| true);
directed_unary!(f32; Exp, exp_round, |_x: f32| true);
directed_unary!(f64; ExpM1, exp_m1_round, |_x: f64| true);
directed_unary!(f32; ExpM1, exp_m1_round, |_x: f32| true);
directed_unary!(f64; Sqrt, sqrt_round, |x: f64| x >= 0.0);
directed_unary!(f32; Sqrt, sqrt_round, |x: f32| x >= 0.0);

macro_rules! directed_binary {
    ($ty:ty; $op:ty, $method:ident, $zero_check:expr) => {
        impl DirectedBinary<$op, $ty> for Mpfr {
            fn eval(lhs: $ty, rhs: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !lhs.is_finite() || !rhs.is_finite() {
                    return Err(non_finite_input());
                }
                if ($zero_check)(rhs) {
                    return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
                }
                let round = to_round(direction);
                let mut output = Float::with_val(<$ty as PrimitiveFloat>::PRECISION, lhs);
                let previous = output.$method(rhs, round);
                output.subnormalize_ieee_round(previous, round);
                finish(output, direction)
            }
        }
    };
}

macro_rules! directed_binary_all {
    ($ty:ty) => {
        directed_binary!($ty; Add, add_assign_round, |_rhs: $ty| false);
        directed_binary!($ty; Sub, sub_assign_round, |_rhs: $ty| false);
        directed_binary!($ty; Mul, mul_assign_round, |_rhs: $ty| false);
        directed_binary!($ty; Div, div_assign_round, |rhs: $ty| rhs == 0.0);
    };
}

directed_binary_all!(f64);
directed_binary_all!(f32);

macro_rules! directed_powi {
    ($ty:ty) => {
        impl DirectedPowI<$ty> for Mpfr {
            fn eval(base: $ty, exponent: i32, direction: Direction) -> Result<Rounded<$ty>> {
                if !base.is_finite() {
                    return Err(non_finite_input());
                }
                let round = to_round(direction);
                let mut output = Float::with_val(<$ty as PrimitiveFloat>::PRECISION, base);
                let previous = output.pow_assign_round(exponent, round);
                output.subnormalize_ieee_round(previous, round);
                finish(output, direction)
            }
        }
    };
}

directed_powi!(f64);
directed_powi!(f32);

macro_rules! convert_exact_to_primitive {
    ($from:ty, $to:ty) => {
        impl Convert<$from, $to> for Mpfr {
            fn convert(value: &$from, direction: Direction) -> Result<Rounded<$to>> {
                let round = to_round(direction);
                let (mut float, previous) =
                    Float::with_val_round(<$to as PrimitiveFloat>::PRECISION, value, round);
                float.subnormalize_ieee_round(previous, round);
                Ok(Rounded::new(
                    <$to as PrimitiveFloat>::from_mpfr(&float, round),
                    direction,
                ))
            }
        }
    };
}

convert_exact_to_primitive!(Rational, f64);
convert_exact_to_primitive!(Rational, f32);
convert_exact_to_primitive!(Integer, f64);
convert_exact_to_primitive!(Integer, f32);
