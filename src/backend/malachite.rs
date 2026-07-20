use malachite::Float;
use malachite::base::num::conversion::traits::RoundingFrom;
use malachite::base::rounding_modes::RoundingMode;

use crate::{
    Add, Backend, Convert, DirectedBinary, DirectedPowI, DirectedUnary, Direction, Div, Error,
    ErrorKind, ExactBinary, ExactUnary, Exp, ExpM1, Ln, Ln1p, Log2, Mul, Neg, Result, Rounded,
    Sqrt, Sub,
};

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

// ---------------------------------------------------------------------------
// Directed primitive operations backed by malachite-float (main branch).
//
// malachite-float's `Float` already implements a correct IEEE conversion to the
// primitive types (`RoundingFrom<Float> for f64/f32`): it saturates to
// +/-inf/+/-max, produces subnormals, and honours signed zero. Because directed
// (Floor/Ceiling) rounding does not suffer double rounding, computing an
// operation at a generous working precision and then converting once with the
// same directed mode yields the correctly rounded f64/f32 result.
// ---------------------------------------------------------------------------

const WORK_PREC: u64 = 128;

fn to_rm(direction: Direction) -> RoundingMode {
    match direction {
        Direction::Down => RoundingMode::Floor,
        Direction::Up => RoundingMode::Ceiling,
        Direction::Nearest => RoundingMode::Nearest,
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::Nearest => Direction::Nearest,
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
    fn is_finite(self) -> bool;
    fn to_float(self) -> Float;
    fn round_from(value: &Float, rm: RoundingMode) -> Self;
    fn min_pos_subnormal() -> Self;
    fn max_finite() -> Self;
    fn native_exp(self) -> Self;
    fn native_exp_m1(self) -> Self;
    fn is_zero(self) -> bool;
    fn next_up_from_neg_one() -> Self;
}

impl PrimitiveFloat for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
    fn to_float(self) -> Float {
        Float::from_primitive_float_prec(self, 64).0
    }
    fn round_from(value: &Float, rm: RoundingMode) -> Self {
        f64::rounding_from(value, rm).0
    }
    fn min_pos_subnormal() -> Self {
        f64::from_bits(1)
    }
    fn max_finite() -> Self {
        f64::MAX
    }
    fn native_exp(self) -> Self {
        self.exp()
    }
    fn native_exp_m1(self) -> Self {
        self.exp_m1()
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn next_up_from_neg_one() -> Self {
        f64::from_bits((-1.0f64).to_bits() - 1)
    }
}

impl PrimitiveFloat for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
    fn to_float(self) -> Float {
        Float::from_primitive_float_prec(self, 32).0
    }
    fn round_from(value: &Float, rm: RoundingMode) -> Self {
        f32::rounding_from(value, rm).0
    }
    fn min_pos_subnormal() -> Self {
        f32::from_bits(1)
    }
    fn max_finite() -> Self {
        f32::MAX
    }
    fn native_exp(self) -> Self {
        self.exp()
    }
    fn native_exp_m1(self) -> Self {
        self.exp_m1()
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn next_up_from_neg_one() -> Self {
        f32::from_bits((-1.0f32).to_bits() - 1)
    }
}

/// Convert a directed malachite result to the primitive type, mapping NaN to a
/// domain error and a non-finite (overflowed) result to an overflow error —
/// matching the MPFR adapter's `finish`.
fn finish<T: PrimitiveFloat>(value: Float, direction: Direction) -> Result<Rounded<T>> {
    if value.is_nan() {
        return Err(domain_error());
    }
    let out = T::round_from(&value, to_rm(direction));
    if !out.is_finite() {
        return Err(overflow_error());
    }
    Ok(Rounded::new(out, direction))
}

macro_rules! directed_unary {
    ($ty:ty; $op:ty, $method:ident, $domain:expr) => {
        impl DirectedUnary<$op, $ty> for Malachite {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if !($domain)(value) {
                    return Err(domain_error());
                }
                let (result, _) = value.to_float().$method(WORK_PREC, to_rm(direction));
                finish::<$ty>(result, direction)
            }
        }
    };
}

directed_unary!(f64; Ln, ln_prec_round, |x: f64| x > 0.0);
directed_unary!(f32; Ln, ln_prec_round, |x: f32| x > 0.0);
directed_unary!(f64; Ln1p, ln_1_plus_x_prec_round, |x: f64| x > -1.0);
directed_unary!(f32; Ln1p, ln_1_plus_x_prec_round, |x: f32| x > -1.0);
directed_unary!(f64; Log2, log_base_2_prec_round, |x: f64| x > 0.0);
directed_unary!(f32; Log2, log_base_2_prec_round, |x: f32| x > 0.0);
directed_unary!(f64; Sqrt, sqrt_prec_round, |x: f64| x >= 0.0);
directed_unary!(f32; Sqrt, sqrt_prec_round, |x: f32| x >= 0.0);

// exp/expm1 series cost grows with |value|, so extreme inputs are resolved from
// the cheap native result rather than driving the backend into a huge range
// reduction. exp is always strictly positive, so an underflowed `Up` bound is
// the least positive value.
macro_rules! directed_exp {
    ($ty:ty) => {
        impl DirectedUnary<Exp, $ty> for Malachite {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                let native = value.native_exp();
                if native.is_zero() {
                    return Ok(Rounded::new(
                        match direction {
                            Direction::Up => <$ty as PrimitiveFloat>::min_pos_subnormal(),
                            _ => 0.0,
                        },
                        direction,
                    ));
                }
                if !native.is_finite() {
                    return match direction {
                        Direction::Down => Ok(Rounded::new(
                            <$ty as PrimitiveFloat>::max_finite(),
                            direction,
                        )),
                        _ => Err(overflow_error()),
                    };
                }
                let (result, _) = value.to_float().exp_prec_round(WORK_PREC, to_rm(direction));
                finish::<$ty>(result, direction)
            }
        }
    };
}

directed_exp!(f64);
directed_exp!(f32);

macro_rules! directed_expm1 {
    ($ty:ty) => {
        impl DirectedUnary<ExpM1, $ty> for Malachite {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                let native = value.native_exp_m1();
                if !native.is_finite() {
                    return match direction {
                        Direction::Down => Ok(Rounded::new(
                            <$ty as PrimitiveFloat>::max_finite(),
                            direction,
                        )),
                        _ => Err(overflow_error()),
                    };
                }
                if native == -1.0 {
                    return Ok(Rounded::new(
                        match direction {
                            Direction::Up => <$ty as PrimitiveFloat>::next_up_from_neg_one(),
                            _ => -1.0,
                        },
                        direction,
                    ));
                }
                let (result, _) = value
                    .to_float()
                    .exp_x_minus_1_prec_round(WORK_PREC, to_rm(direction));
                finish::<$ty>(result, direction)
            }
        }
    };
}

directed_expm1!(f64);
directed_expm1!(f32);

macro_rules! directed_binary {
    ($ty:ty; $op:ty, $method:ident, $divzero:expr) => {
        impl DirectedBinary<$op, $ty> for Malachite {
            fn eval(lhs: $ty, rhs: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !lhs.is_finite() || !rhs.is_finite() {
                    return Err(non_finite_input());
                }
                if ($divzero)(rhs) {
                    return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
                }
                let (result, _) =
                    lhs.to_float()
                        .$method(rhs.to_float(), WORK_PREC, to_rm(direction));
                finish::<$ty>(result, direction)
            }
        }
    };
}

macro_rules! directed_binary_all {
    ($ty:ty) => {
        directed_binary!($ty; Add, add_prec_round, |_b: $ty| false);
        directed_binary!($ty; Sub, sub_prec_round, |_b: $ty| false);
        directed_binary!($ty; Mul, mul_prec_round, |_b: $ty| false);
        directed_binary!($ty; Div, div_prec_round, |b: $ty| b == 0.0);
    };
}

directed_binary_all!(f64);
directed_binary_all!(f32);

// powi(base, n) = sign * |base|^n. Negating flips the rounding direction, so a
// negative base with an odd exponent is computed in the opposite direction and
// then negated.
macro_rules! directed_powi {
    ($ty:ty) => {
        impl DirectedPowI<$ty> for Malachite {
            fn eval(base: $ty, exponent: &i32, direction: Direction) -> Result<Rounded<$ty>> {
                if !base.is_finite() {
                    return Err(non_finite_input());
                }
                if base == 0.0 && *exponent < 0 {
                    return Err(Error::new(
                        ErrorKind::DivisionByZero,
                        "zero to a negative power",
                    ));
                }
                // Use the sign bit, not `< 0.0`, so `-0.0` raised to an odd power
                // keeps its negative sign (matching IEEE / MPFR).
                let negate = base.is_sign_negative() && exponent % 2 != 0;
                let compute_direction = if negate {
                    opposite(direction)
                } else {
                    direction
                };
                let abs_base = base.abs().to_float();
                let exp_float = malachite::Integer::from(*exponent).to_float();
                let (mut result, _) =
                    abs_base.pow_prec_round(exp_float, WORK_PREC, to_rm(compute_direction));
                if negate {
                    result = -result;
                }
                finish::<$ty>(result, direction)
            }
        }
    };
}

directed_powi!(f64);
directed_powi!(f32);

// Exact-number conversions use malachite's direct, single correctly-rounded
// `RoundingFrom` (no `Float` intermediate, so no Nearest double rounding). They
// saturate to +/-inf on overflow with no error, matching the MPFR adapter.
macro_rules! convert_to_primitive {
    ($from:ty, $to:ty) => {
        impl Convert<$from, $to> for Malachite {
            fn convert(value: &$from, direction: Direction) -> Result<Rounded<$to>> {
                let out = <$to as RoundingFrom<&$from>>::rounding_from(value, to_rm(direction)).0;
                Ok(Rounded::new(out, direction))
            }
        }
    };
}

convert_to_primitive!(malachite::Rational, f64);
convert_to_primitive!(malachite::Rational, f32);
convert_to_primitive!(malachite::Integer, f64);
convert_to_primitive!(malachite::Integer, f32);
convert_to_primitive!(malachite::Natural, f64);
convert_to_primitive!(malachite::Natural, f32);

// A malachite Integer needs to become a Float for powi.
trait IntegerToFloat {
    fn to_float(self) -> Float;
}
impl IntegerToFloat for malachite::Integer {
    fn to_float(self) -> Float {
        Float::from_integer_prec(self, 64).0
    }
}
