use core::panic::UnwindSafe;

use dashu::{
    base::{Approximation, EstimatedLog2, Sign},
    float::{
        FBig,
        round::{
            Rounding,
            mode::{Down, HalfEven, Up},
        },
    },
    integer::{IBig, UBig},
    rational::RBig,
};

use crate::{
    Add, Backend, CheckedBinary, Convert, DirectedBinary, DirectedPowI, DirectedUnary, Direction,
    Div, Error, ErrorKind, ExactBinary, ExactUnary, Exp, ExpM1, FromParts, IntoParts, Ln, Ln1p,
    Log2, Mul, Neg, Result, Rounded, Sqrt, Sub,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Dashu;

impl Backend for Dashu {
    type Natural = UBig;
    type Integer = IBig;
    type Rational = RBig;
}

macro_rules! exact_binary {
    ($ty:ty; $($op:ty, $trait:ident, $method:ident);+ $(;)?) => {
        $(impl ExactBinary<$op, $ty> for Dashu {
            #[inline]
            fn eval(lhs: &$ty, rhs: &$ty) -> $ty {
                core::ops::$trait::$method(lhs, rhs)
            }
        })+
    };
}

exact_binary!(UBig; Add, Add, add; Mul, Mul, mul);
exact_binary!(IBig; Add, Add, add; Sub, Sub, sub; Mul, Mul, mul);
exact_binary!(RBig; Add, Add, add; Sub, Sub, sub; Mul, Mul, mul);

impl ExactUnary<Neg, IBig> for Dashu {
    #[inline]
    fn eval(value: &IBig) -> IBig {
        -value
    }
}

impl ExactUnary<Neg, RBig> for Dashu {
    #[inline]
    fn eval(value: &RBig) -> RBig {
        -value
    }
}

impl CheckedBinary<Div, RBig> for Dashu {
    fn eval(lhs: &RBig, rhs: &RBig) -> Result<RBig> {
        if rhs.is_zero() {
            return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
        }

        // Construct the quotient from parts rather than delegating to RBig `/`.
        // This keeps the abstraction correct on Dashu releases affected by
        // https://github.com/cmpute/dashu/issues/57.
        let (lhs_num, lhs_den) = lhs.clone().into_parts();
        let (rhs_num, rhs_den) = rhs.clone().into_parts();
        let (rhs_sign, rhs_abs) = rhs_num.into_parts();

        let mut numerator = lhs_num * rhs_den;
        if rhs_sign == Sign::Negative {
            numerator = -numerator;
        }
        let denominator = lhs_den * rhs_abs;
        Ok(RBig::from_parts(numerator, denominator))
    }
}

impl FromParts<RBig, IBig, UBig> for Dashu {
    fn from_parts(numerator: IBig, denominator: UBig) -> Result<RBig> {
        if denominator.is_zero() {
            return Err(Error::new(
                ErrorKind::DivisionByZero,
                "zero rational denominator",
            ));
        }
        Ok(RBig::from_parts(numerator, denominator))
    }
}

impl IntoParts<RBig, IBig, UBig> for Dashu {
    #[inline]
    fn into_parts(value: RBig) -> (IBig, UBig) {
        value.into_parts()
    }
}

fn catch_backend<T>(f: impl FnOnce() -> T + UnwindSafe) -> Result<T> {
    std::panic::catch_unwind(f).map_err(|_| {
        Error::new(
            ErrorKind::Backend,
            "Dashu panicked while evaluating an operation",
        )
    })
}

trait PrimitiveOutput: Copy {
    const PRECISION: usize;

    fn from_up(value: FBig<Up>) -> Self;
    fn from_down(value: FBig<Down>) -> Self;
    fn from_nearest(value: FBig<HalfEven>) -> Self;
    fn min_positive_subnormal() -> Self;
}

impl PrimitiveOutput for f64 {
    const PRECISION: usize = f64::MANTISSA_DIGITS as usize;

    fn from_up(value: FBig<Up>) -> Self {
        match value.to_f64() {
            Approximation::Exact(v) | Approximation::Inexact(v, Rounding::AddOne) => v,
            Approximation::Inexact(v, _) => next_up_f64(v),
        }
    }

    fn from_down(value: FBig<Down>) -> Self {
        match value.to_f64() {
            Approximation::Exact(v) | Approximation::Inexact(v, Rounding::SubOne) => v,
            Approximation::Inexact(v, _) => next_down_f64(v),
        }
    }

    fn from_nearest(value: FBig<HalfEven>) -> Self {
        value.to_f64().value()
    }

    fn min_positive_subnormal() -> Self {
        f64::from_bits(1)
    }
}

impl PrimitiveOutput for f32 {
    const PRECISION: usize = f32::MANTISSA_DIGITS as usize;

    fn from_up(value: FBig<Up>) -> Self {
        match value.to_f32() {
            Approximation::Exact(v) | Approximation::Inexact(v, Rounding::AddOne) => v,
            Approximation::Inexact(v, _) => next_up_f32(v),
        }
    }

    fn from_down(value: FBig<Down>) -> Self {
        match value.to_f32() {
            Approximation::Exact(v) | Approximation::Inexact(v, Rounding::SubOne) => v,
            Approximation::Inexact(v, _) => next_down_f32(v),
        }
    }

    fn from_nearest(value: FBig<HalfEven>) -> Self {
        value.to_f32().value()
    }

    fn min_positive_subnormal() -> Self {
        f32::from_bits(1)
    }
}

fn next_up_f64(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

fn next_down_f64(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits((1u64 << 63) | 1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up_f32(value: f32) -> f32 {
    if value.is_nan() || value == f32::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f32::from_bits(1);
    }
    let bits = value.to_bits();
    f32::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

fn next_down_f32(value: f32) -> f32 {
    if value.is_nan() || value == f32::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f32::from_bits((1u32 << 31) | 1);
    }
    let bits = value.to_bits();
    f32::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
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

macro_rules! directed_unary {
    ($ty:ty; $op:ty, $method:ident, $domain:expr, $native:expr) => {
        impl DirectedUnary<$op, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if !($domain)(value) {
                    return Err(domain_error());
                }
                let native = ($native)(value);
                if native.is_nan() {
                    return Err(domain_error());
                }
                if native.is_infinite() {
                    return Err(overflow_error());
                }

                let output = match direction {
                    Direction::Up => {
                        let input = FBig::<Up>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        <$ty as PrimitiveOutput>::from_up(catch_backend(|| input.$method())?)
                    }
                    Direction::Down => {
                        let input = FBig::<Down>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        <$ty as PrimitiveOutput>::from_down(catch_backend(|| input.$method())?)
                    }
                    Direction::Nearest => {
                        let input = FBig::<HalfEven>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        <$ty as PrimitiveOutput>::from_nearest(catch_backend(|| input.$method())?)
                    }
                };

                if !output.is_finite() {
                    return Err(overflow_error());
                }
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

macro_rules! directed_exp {
    ($ty:ty) => {
        impl DirectedUnary<Exp, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if value.exp().is_infinite() {
                    return Err(overflow_error());
                }

                let output = match direction {
                    Direction::Up => {
                        let input = FBig::<Up>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        match catch_backend(|| input.exp()) {
                            Ok(value) => <$ty as PrimitiveOutput>::from_up(value),
                            Err(_) if value.is_sign_negative() => {
                                <$ty as PrimitiveOutput>::min_positive_subnormal()
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    Direction::Down => {
                        let input = FBig::<Down>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        match catch_backend(|| input.exp()) {
                            Ok(value) => <$ty as PrimitiveOutput>::from_down(value),
                            Err(_) if value.is_sign_negative() => 0.0,
                            Err(error) => return Err(error),
                        }
                    }
                    Direction::Nearest => {
                        let input = FBig::<HalfEven>::try_from(value)
                            .map_err(|_| non_finite_input())?
                            .with_precision(<$ty as PrimitiveOutput>::PRECISION)
                            .value();
                        match catch_backend(|| input.exp()) {
                            Ok(value) => <$ty as PrimitiveOutput>::from_nearest(value),
                            Err(_) if value.is_sign_negative() => 0.0,
                            Err(error) => return Err(error),
                        }
                    }
                };

                if !output.is_finite() {
                    return Err(overflow_error());
                }
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

directed_unary!(f64; Ln, ln, |x: f64| x > 0.0, |x: f64| x.ln());
directed_unary!(f32; Ln, ln, |x: f32| x > 0.0, |x: f32| x.ln());
directed_unary!(f64; Ln1p, ln_1p, |x: f64| x > -1.0, |x: f64| x.ln_1p());
directed_unary!(f32; Ln1p, ln_1p, |x: f32| x > -1.0, |x: f32| x.ln_1p());
directed_unary!(f64; ExpM1, exp_m1, |_x: f64| true, |x: f64| x.exp_m1());
directed_unary!(f32; ExpM1, exp_m1, |_x: f32| true, |x: f32| x.exp_m1());
directed_unary!(f64; Sqrt, sqrt, |x: f64| x >= 0.0, |x: f64| x.sqrt());
directed_unary!(f32; Sqrt, sqrt, |x: f32| x >= 0.0, |x: f32| x.sqrt());
directed_exp!(f64);
directed_exp!(f32);

trait DashuLog2 {
    fn op_log2(self) -> Self;
}

impl DashuLog2 for FBig<Down> {
    fn op_log2(self) -> Self {
        Self::try_from(self.log2_bounds().0).unwrap()
    }
}

impl DashuLog2 for FBig<Up> {
    fn op_log2(self) -> Self {
        Self::try_from(self.log2_bounds().1).unwrap()
    }
}

macro_rules! directed_log2 {
    ($ty:ty) => {
        impl DirectedUnary<Log2, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if value <= 0.0 {
                    return Err(domain_error());
                }

                let output = match direction {
                    Direction::Up => {
                        let input = FBig::<Up>::try_from(value).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_up(catch_backend(|| input.op_log2())?)
                    }
                    Direction::Down => {
                        let input =
                            FBig::<Down>::try_from(value).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_down(catch_backend(|| input.op_log2())?)
                    }
                    Direction::Nearest => {
                        return Err(Error::new(
                            ErrorKind::Unsupported,
                            "nearest log2 rounding is not implemented for Dashu",
                        ));
                    }
                };
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

directed_log2!(f64);
directed_log2!(f32);

macro_rules! directed_binary {
    ($ty:ty; $op:ty, $operator:tt, $native:expr, $zero_check:expr) => {
        impl DirectedBinary<$op, $ty> for Dashu {
            fn eval(lhs: $ty, rhs: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !lhs.is_finite() || !rhs.is_finite() {
                    return Err(non_finite_input());
                }
                if ($zero_check)(rhs) {
                    return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
                }
                let native = ($native)(lhs, rhs);
                if native.is_nan() {
                    return Err(domain_error());
                }
                if native.is_infinite() {
                    return Err(overflow_error());
                }

                let output = match direction {
                    Direction::Up => {
                        let lhs = FBig::<Up>::try_from(lhs).map_err(|_| non_finite_input())?;
                        let rhs = FBig::<Up>::try_from(rhs).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_up(catch_backend(|| lhs $operator rhs)?)
                    }
                    Direction::Down => {
                        let lhs = FBig::<Down>::try_from(lhs).map_err(|_| non_finite_input())?;
                        let rhs = FBig::<Down>::try_from(rhs).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_down(catch_backend(|| lhs $operator rhs)?)
                    }
                    Direction::Nearest => {
                        let lhs = FBig::<HalfEven>::try_from(lhs).map_err(|_| non_finite_input())?;
                        let rhs = FBig::<HalfEven>::try_from(rhs).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_nearest(catch_backend(|| lhs $operator rhs)?)
                    }
                };
                if !output.is_finite() {
                    return Err(overflow_error());
                }
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

macro_rules! directed_binary_all {
    ($ty:ty) => {
        directed_binary!($ty; Add, +, |a: $ty, b: $ty| a + b, |_b: $ty| false);
        directed_binary!($ty; Sub, -, |a: $ty, b: $ty| a - b, |_b: $ty| false);
        directed_binary!($ty; Mul, *, |a: $ty, b: $ty| a * b, |_b: $ty| false);
        directed_binary!($ty; Div, /, |a: $ty, b: $ty| a / b, |b: $ty| b == 0.0);
    };
}

directed_binary_all!(f64);
directed_binary_all!(f32);

macro_rules! directed_powi {
    ($ty:ty) => {
        impl DirectedPowI<$ty> for Dashu {
            fn eval(base: $ty, exponent: i32, direction: Direction) -> Result<Rounded<$ty>> {
                if !base.is_finite() {
                    return Err(non_finite_input());
                }
                let native = base.powi(exponent);
                if native.is_nan() {
                    return Err(domain_error());
                }
                if native.is_infinite() {
                    return Err(overflow_error());
                }
                let exponent = IBig::from(exponent);
                let output = match direction {
                    Direction::Up => {
                        let base = FBig::<Up>::try_from(base).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_up(catch_backend(|| {
                            base.powi(exponent.clone())
                        })?)
                    }
                    Direction::Down => {
                        let base = FBig::<Down>::try_from(base).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_down(catch_backend(|| {
                            base.powi(exponent.clone())
                        })?)
                    }
                    Direction::Nearest => {
                        let base =
                            FBig::<HalfEven>::try_from(base).map_err(|_| non_finite_input())?;
                        <$ty as PrimitiveOutput>::from_nearest(catch_backend(|| {
                            base.powi(exponent)
                        })?)
                    }
                };
                if !output.is_finite() {
                    return Err(overflow_error());
                }
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

directed_powi!(f64);
directed_powi!(f32);

macro_rules! convert_to_primitive {
    ($from:ty, $to:ty, $method:ident, $next_up:ident, $next_down:ident) => {
        impl Convert<$from, $to> for Dashu {
            fn convert(value: &$from, direction: Direction) -> Result<Rounded<$to>> {
                let approximation = value.$method();
                let rounded = match (direction, approximation) {
                    (_, Approximation::Exact(v)) => return Ok(Rounded::new(v, direction)),
                    (Direction::Up, Approximation::Inexact(v, Sign::Positive)) => v,
                    (Direction::Up, Approximation::Inexact(v, _)) => $next_up(v),
                    (Direction::Down, Approximation::Inexact(v, Sign::Negative)) => v,
                    (Direction::Down, Approximation::Inexact(v, _)) => $next_down(v),
                    (Direction::Nearest, Approximation::Inexact(v, _)) => v,
                };
                Ok(Rounded::new(rounded, direction))
            }
        }
    };
}

convert_to_primitive!(RBig, f64, to_f64, next_up_f64, next_down_f64);
convert_to_primitive!(RBig, f32, to_f32, next_up_f32, next_down_f32);
convert_to_primitive!(IBig, f64, to_f64, next_up_f64, next_down_f64);
convert_to_primitive!(IBig, f32, to_f32, next_up_f32, next_down_f32);
convert_to_primitive!(UBig, f64, to_f64, next_up_f64, next_down_f64);
convert_to_primitive!(UBig, f32, to_f32, next_up_f32, next_down_f32);
