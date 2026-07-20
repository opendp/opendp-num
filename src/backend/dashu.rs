use core::panic::UnwindSafe;

use dashu::{
    base::{BitTest, EstimatedLog2},
    float::{
        FBig,
        round::mode::{Down, HalfEven, Up},
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
        Ok(lhs / rhs)
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

fn non_finite_input() -> Error {
    Error::new(ErrorKind::NonFinite, "input must be finite")
}

fn domain_error() -> Error {
    Error::new(ErrorKind::Domain, "input is outside the operation domain")
}

fn overflow_error() -> Error {
    Error::new(ErrorKind::Overflow, "operation produced a non-finite value")
}

fn extreme_power<T: PrimitiveRound + SignBit>(
    negative: bool,
    overflow: bool,
    direction: Direction,
) -> Result<Rounded<T>> {
    if overflow {
        return match direction {
            Direction::Down if !negative => Ok(Rounded::new(T::max_finite(), direction)),
            Direction::Up if negative => Ok(Rounded::new(T::neg_max_finite(), direction)),
            _ => Err(overflow_error()),
        };
    }

    let output = match direction {
        Direction::Nearest => T::signed_zero(negative),
        Direction::Down if negative => T::min_pos_subnormal().neg(),
        Direction::Down => T::signed_zero(false),
        Direction::Up if negative => T::signed_zero(true),
        Direction::Up => T::min_pos_subnormal(),
    };
    Ok(Rounded::new(output, direction))
}

// ---------------------------------------------------------------------------
// Correctly rounded directed conversion of an exact rational to a primitive
// float.
//
// The result is decided purely by exact `RBig` comparison against candidate
// grid points. It never trusts `to_f64`/`to_f32`'s `Approximation` tag (which
// dashu can misreport, see findings DASHU-015) and it clamps correctly at
// signed zero and the subnormal boundary, so a value that underflows toward
// zero never crosses to the opposite sign.
// ---------------------------------------------------------------------------

fn rbig_zero() -> RBig {
    RBig::from(IBig::from(0))
}

trait PrimitiveRound: Copy {
    /// Target mantissa precision in bits (used for transcendental working precision).
    const PRECISION: usize;

    fn max_finite() -> Self;
    fn neg_max_finite() -> Self;
    fn min_pos_subnormal() -> Self;
    fn infinity(negative: bool) -> Self;
    fn signed_zero(negative: bool) -> Self;
    /// A finite starting candidate near `q`; correctness comes from the caller's
    /// exact comparison walk, so a non-finite seed is clamped by the caller.
    fn seed(q: &RBig) -> Self;
    /// Exact rational value, or `None` if the float is non-finite.
    fn to_rat(self) -> Option<RBig>;
    fn next_up(self) -> Self;
    fn next_down(self) -> Self;
    fn is_finite(self) -> bool;
    fn is_zero(self) -> bool;
    /// Identity bits, zero-extended for `f32`, for stability comparison.
    fn ident_bits(self) -> u64;
    /// Low mantissa bit, for round-half-to-even ties.
    fn low_bit_set(self) -> bool;
}

impl PrimitiveRound for f64 {
    const PRECISION: usize = f64::MANTISSA_DIGITS as usize;

    fn max_finite() -> Self {
        f64::MAX
    }
    fn neg_max_finite() -> Self {
        f64::MIN
    }
    fn min_pos_subnormal() -> Self {
        f64::from_bits(1)
    }
    fn infinity(negative: bool) -> Self {
        if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    }
    fn signed_zero(negative: bool) -> Self {
        if negative { -0.0 } else { 0.0 }
    }
    fn seed(q: &RBig) -> Self {
        q.to_f64().value()
    }
    fn to_rat(self) -> Option<RBig> {
        if !self.is_finite() {
            return None;
        }
        let b = self.to_bits();
        let sign = if b >> 63 == 1 { -1i8 } else { 1 };
        let exp = ((b >> 52) & 0x7ff) as i64;
        let mant = b & 0x000f_ffff_ffff_ffff;
        let (m, e) = if exp == 0 {
            (mant, -1074i64)
        } else {
            (mant | 0x0010_0000_0000_0000, exp - 1075)
        };
        Some(rat_from_scaled(IBig::from(m) * IBig::from(sign), e))
    }
    fn next_up(self) -> Self {
        next_up_f64(self)
    }
    fn next_down(self) -> Self {
        next_down_f64(self)
    }
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn ident_bits(self) -> u64 {
        self.to_bits()
    }
    fn low_bit_set(self) -> bool {
        self.to_bits() & 1 == 1
    }
}

impl PrimitiveRound for f32 {
    const PRECISION: usize = f32::MANTISSA_DIGITS as usize;

    fn max_finite() -> Self {
        f32::MAX
    }
    fn neg_max_finite() -> Self {
        f32::MIN
    }
    fn min_pos_subnormal() -> Self {
        f32::from_bits(1)
    }
    fn infinity(negative: bool) -> Self {
        if negative {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        }
    }
    fn signed_zero(negative: bool) -> Self {
        if negative { -0.0 } else { 0.0 }
    }
    fn seed(q: &RBig) -> Self {
        q.to_f32().value()
    }
    fn to_rat(self) -> Option<RBig> {
        if !self.is_finite() {
            return None;
        }
        let b = self.to_bits();
        let sign = if b >> 31 == 1 { -1i8 } else { 1 };
        let exp = ((b >> 23) & 0xff) as i64;
        let mant = (b & 0x007f_ffff) as u64;
        let (m, e) = if exp == 0 {
            (mant, -149i64)
        } else {
            (mant | 0x0080_0000, exp - 150)
        };
        Some(rat_from_scaled(IBig::from(m) * IBig::from(sign), e))
    }
    fn next_up(self) -> Self {
        next_up_f32(self)
    }
    fn next_down(self) -> Self {
        next_down_f32(self)
    }
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn ident_bits(self) -> u64 {
        self.to_bits() as u64
    }
    fn low_bit_set(self) -> bool {
        self.to_bits() & 1 == 1
    }
}

/// `significand * 2^exponent` as an exact rational.
fn rat_from_scaled(significand: IBig, exponent: i64) -> RBig {
    if exponent >= 0 {
        RBig::from(significand << (exponent as usize))
    } else {
        RBig::from_parts(significand, UBig::from(1u8) << ((-exponent) as usize))
    }
}

/// Correctly rounded directed conversion of `q` to `T` (infallible, IEEE).
///
/// `Up` returns the least value `>= q`; `Down` the greatest value `<= q`;
/// `Nearest` the closest with ties to even. Out-of-range results saturate to
/// `±inf`/`±max` exactly as IEEE directed rounding requires (matching MPFR's
/// `to_fXX_round`); callers that must reject a non-finite bound apply their own
/// finite check. `zero_negative` supplies the sign for an exact-zero result,
/// which the rational value alone cannot carry.
fn round_rational<T: PrimitiveRound>(q: &RBig, direction: Direction, zero_negative: bool) -> T {
    let zero = rbig_zero();
    if *q == zero {
        return T::signed_zero(zero_negative);
    }
    let negative = *q < zero;
    let max = T::max_finite();
    let max_rat = max.to_rat().expect("max finite");

    // Out-of-range: rounding toward the infinity saturates there; away saturates
    // at the finite extreme; nearest crosses to infinity past the half-ulp point.
    if *q > max_rat {
        return match direction {
            Direction::Down => max,
            Direction::Up => T::infinity(false),
            Direction::Nearest => {
                let half_ulp = (&max_rat - &max.next_down().to_rat().expect("finite")) / rbig_two();
                if (q - &max_rat) > half_ulp {
                    T::infinity(false)
                } else {
                    max
                }
            }
        };
    }
    let neg_max = T::neg_max_finite();
    let neg_max_rat = neg_max.to_rat().expect("max finite");
    if *q < neg_max_rat {
        return match direction {
            Direction::Up => neg_max,
            Direction::Down => T::infinity(true),
            Direction::Nearest => {
                let half_ulp =
                    (&neg_max.next_up().to_rat().expect("finite") - &neg_max_rat) / rbig_two();
                if (&neg_max_rat - q) > half_ulp {
                    T::infinity(true)
                } else {
                    neg_max
                }
            }
        };
    }

    // In range: bracket q with lo = greatest representable <= q.
    let mut c = T::seed(q);
    if !c.is_finite() {
        c = if negative { neg_max } else { max };
    }
    while c.to_rat().map(|r| r > *q).unwrap_or(false) {
        c = c.next_down();
    }
    loop {
        let n = c.next_up();
        match n.to_rat() {
            Some(nr) if nr <= *q => c = n,
            _ => break,
        }
    }

    let lo = c;
    let lo_rat = lo.to_rat().expect("lo finite");
    if lo_rat == *q {
        return lo;
    }
    let fix_zero = |v: T| {
        if v.is_zero() {
            T::signed_zero(negative)
        } else {
            v
        }
    };

    match direction {
        Direction::Down => fix_zero(lo),
        Direction::Up => fix_zero(lo.next_up()),
        Direction::Nearest => {
            let hi = lo.next_up();
            let hi_rat = hi.to_rat().expect("hi finite in range");
            let dist_lo = q - &lo_rat;
            let dist_hi = &hi_rat - q;
            let pick = if dist_lo < dist_hi {
                lo
            } else if dist_hi < dist_lo {
                hi
            } else if lo.low_bit_set() {
                hi
            } else {
                lo
            };
            fix_zero(pick)
        }
    }
}

fn rbig_two() -> RBig {
    RBig::from(IBig::from(2))
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

/// Exact rational of a raw base-2 `FBig` (always exact — a binary float is rational).
fn fbig_rat<R: dashu::float::round::Round>(value: &FBig<R>) -> RBig {
    let repr = value.repr();
    let significand = repr.significand().clone();
    let exponent = repr.exponent();
    if exponent >= 0 {
        RBig::from(significand << (exponent as usize))
    } else {
        RBig::from_parts(significand, UBig::from(1u8) << ((-exponent) as usize))
    }
}

/// Adaptive-precision directed evaluation of a transcendental.
///
/// `raw_at(prec, direction)` returns the exact rational of the backend result
/// computed with the matching directed rounding mode at `prec` bits. Precision
/// is doubled until the directed f64/f32 rounding stabilises, which lets
/// underflow cases resolve to the correct side of zero (findings DASHU-006/013)
/// rather than being limited by the target-precision working set.
fn eval_adaptive<T: PrimitiveRound>(
    direction: Direction,
    start_prec: usize,
    zero_negative: bool,
    raw_at: impl Fn(usize, Direction) -> Result<RBig>,
) -> Result<Rounded<T>> {
    const CAP: usize = 4096;
    let mut prec = start_prec.clamp(T::PRECISION * 2, CAP);
    let mut previous: Option<u64> = None;
    loop {
        let raw = raw_at(prec, direction)?;
        let rounded = round_rational::<T>(&raw, direction, zero_negative);
        let bits = rounded.ident_bits();
        if previous == Some(bits) || prec >= CAP {
            // Directed transcendental/power: a non-finite bound is an overflow.
            if !rounded.is_finite() {
                return Err(overflow_error());
            }
            return Ok(Rounded::new(rounded, direction));
        }
        previous = Some(bits);
        prec = (prec * 2).min(CAP);
    }
}

/// Working precision that starts past the plateau where a directed transcendental
/// returns its argument unchanged. For inputs near zero the correction term for
/// ln1p/expm1 sits ~|log2(value)| bits below the result, so the base precision is
/// scaled by that magnitude (findings DASHU-006/013).
fn start_precision(value_log2_magnitude: usize) -> usize {
    128 + value_log2_magnitude
}

macro_rules! directed_unary {
    ($ty:ty; $op:ty, $method:ident, $domain:expr, $zero_negative:expr) => {
        impl DirectedUnary<$op, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                if !($domain)(value) {
                    return Err(domain_error());
                }
                let magnitude = if value == 0.0 {
                    0usize
                } else {
                    let l = value.abs().log2();
                    if l.is_finite() {
                        l.abs().ceil() as usize
                    } else {
                        0
                    }
                };
                let zero_negative = ($zero_negative)(value);
                eval_adaptive::<$ty>(
                    direction,
                    start_precision(magnitude),
                    zero_negative,
                    |prec, dir| {
                        let raw = match dir {
                            Direction::Up => {
                                let x = FBig::<Up>::try_from(value)
                                    .map_err(|_| non_finite_input())?
                                    .with_precision(prec)
                                    .value();
                                fbig_rat(&catch_backend(|| x.$method())?)
                            }
                            Direction::Down => {
                                let x = FBig::<Down>::try_from(value)
                                    .map_err(|_| non_finite_input())?
                                    .with_precision(prec)
                                    .value();
                                fbig_rat(&catch_backend(|| x.$method())?)
                            }
                            Direction::Nearest => {
                                let x = FBig::<HalfEven>::try_from(value)
                                    .map_err(|_| non_finite_input())?
                                    .with_precision(prec)
                                    .value();
                                fbig_rat(&catch_backend(|| x.$method())?)
                            }
                        };
                        Ok(raw)
                    },
                )
            }
        }
    };
}

// ln(1) is exactly +0; ln1p and sqrt preserve the sign of a zero result.
directed_unary!(f64; Ln, ln, |x: f64| x > 0.0, |_x: f64| false);
directed_unary!(f32; Ln, ln, |x: f32| x > 0.0, |_x: f32| false);
directed_unary!(f64; Ln1p, ln_1p, |x: f64| x > -1.0, |x: f64| x.is_sign_negative());
directed_unary!(f32; Ln1p, ln_1p, |x: f32| x > -1.0, |x: f32| x.is_sign_negative());
directed_unary!(f64; Sqrt, sqrt, |x: f64| x >= 0.0, |x: f64| x.is_sign_negative());
directed_unary!(f32; Sqrt, sqrt, |x: f32| x >= 0.0, |x: f32| x.is_sign_negative());

/// |log2(value)| as a working-precision boost, so results near 1 (tiny input)
/// or near a subnormal boundary resolve to the correct side.
fn input_magnitude<T: Into<f64>>(value: T) -> usize {
    let v: f64 = value.into();
    if v == 0.0 {
        return 0;
    }
    let l = v.abs().log2();
    if l.is_finite() {
        l.abs().ceil() as usize
    } else {
        0
    }
}

macro_rules! transcendental_raw {
    ($value:expr, $method:ident) => {
        |prec, dir| -> Result<RBig> {
            Ok(match dir {
                Direction::Up => {
                    let x = FBig::<Up>::try_from($value)
                        .map_err(|_| non_finite_input())?
                        .with_precision(prec)
                        .value();
                    fbig_rat(&catch_backend(|| x.$method())?)
                }
                Direction::Down => {
                    let x = FBig::<Down>::try_from($value)
                        .map_err(|_| non_finite_input())?
                        .with_precision(prec)
                        .value();
                    fbig_rat(&catch_backend(|| x.$method())?)
                }
                Direction::Nearest => {
                    let x = FBig::<HalfEven>::try_from($value)
                        .map_err(|_| non_finite_input())?
                        .with_precision(prec)
                        .value();
                    fbig_rat(&catch_backend(|| x.$method())?)
                }
            })
        }
    };
}

// exp/expm1 series cost grows with |value|, so extreme inputs are resolved from
// the cheap native result (which saturates predictably) instead of driving the
// arbitrary-precision backend into a multi-gigabyte range reduction. The normal
// range (|value| within a few hundred) is where the backend is affordable.
//
// exp is always strictly positive; when it underflows to zero that sign is lost,
// so an `Up` bound must still be the least positive value (finding DASHU-004).
macro_rules! directed_exp {
    ($ty:ty) => {
        impl DirectedUnary<Exp, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                let native: $ty = value.exp();
                if native == 0.0 {
                    // Underflow: true value is in (0, min_subnormal).
                    let output = match direction {
                        Direction::Up => <$ty as PrimitiveRound>::min_pos_subnormal(),
                        _ => <$ty as PrimitiveRound>::signed_zero(false),
                    };
                    return Ok(Rounded::new(output, direction));
                }
                if !native.is_finite() {
                    // Overflow: true value exceeds max_finite.
                    return match direction {
                        Direction::Down => Ok(Rounded::new(
                            <$ty as PrimitiveRound>::max_finite(),
                            direction,
                        )),
                        _ => Err(overflow_error()),
                    };
                }
                // exp is never exactly zero for finite input (underflow handled above).
                eval_adaptive::<$ty>(
                    direction,
                    start_precision(input_magnitude(value)),
                    false,
                    transcendental_raw!(value, exp),
                )
            }
        }
    };
}

directed_exp!(f64);
directed_exp!(f32);

// expm1 saturates to +inf for large positive input and to -1 (from above) for
// large negative input; both are the cheap-native regions that would otherwise
// drive an unbounded series.
macro_rules! directed_expm1 {
    ($ty:ty) => {
        impl DirectedUnary<ExpM1, $ty> for Dashu {
            fn eval(value: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !value.is_finite() {
                    return Err(non_finite_input());
                }
                let native: $ty = value.exp_m1();
                if !native.is_finite() {
                    return match direction {
                        Direction::Down => Ok(Rounded::new(
                            <$ty as PrimitiveRound>::max_finite(),
                            direction,
                        )),
                        _ => Err(overflow_error()),
                    };
                }
                if native == -1.0 {
                    // exp(value) has fully underflowed: true value is in (-1, -1+eps].
                    let output = match direction {
                        Direction::Up => <$ty as PrimitiveRound>::next_up(-1.0),
                        _ => -1.0,
                    };
                    return Ok(Rounded::new(output, direction));
                }
                // expm1 preserves the sign of a zero argument (expm1(-0) = -0).
                eval_adaptive::<$ty>(
                    direction,
                    start_precision(input_magnitude(value)),
                    value.is_sign_negative(),
                    transcendental_raw!(value, exp_m1),
                )
            }
        }
    };
}

directed_expm1!(f64);
directed_expm1!(f32);

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
                // dashu-float exposes only interval bounds for log2, so the
                // directed result is the correctly rounded conversion of the
                // sound bound on the requested side. This is a valid directed
                // bound but not necessarily the tightest (finding DASHU-007).
                let bound = match direction {
                    Direction::Down => {
                        let x = FBig::<Down>::try_from(value).map_err(|_| non_finite_input())?;
                        let fb = FBig::<Down>::try_from(catch_backend(|| x.log2_bounds().0)?)
                            .map_err(|_| domain_error())?;
                        fbig_rat(&fb)
                    }
                    Direction::Up => {
                        let x = FBig::<Up>::try_from(value).map_err(|_| non_finite_input())?;
                        let fb = FBig::<Up>::try_from(catch_backend(|| x.log2_bounds().1)?)
                            .map_err(|_| domain_error())?;
                        fbig_rat(&fb)
                    }
                    Direction::Nearest => {
                        return Err(Error::new(
                            ErrorKind::Unsupported,
                            "nearest log2 rounding is not implemented for Dashu",
                        ));
                    }
                };
                Ok(Rounded::new(
                    round_rational::<$ty>(&bound, direction, false),
                    direction,
                ))
            }
        }
    };
}

directed_log2!(f64);
directed_log2!(f32);

macro_rules! directed_binary {
    ($ty:ty; $op:ty, $operator:tt, $divzero:expr, $zero_negative:expr) => {
        impl DirectedBinary<$op, $ty> for Dashu {
            fn eval(lhs: $ty, rhs: $ty, direction: Direction) -> Result<Rounded<$ty>> {
                if !lhs.is_finite() || !rhs.is_finite() {
                    return Err(non_finite_input());
                }
                if ($divzero)(rhs) {
                    return Err(Error::new(ErrorKind::DivisionByZero, "division by zero"));
                }
                // Exact rational arithmetic (RBig `/` is correct in dashu 0.5),
                // then a single correctly rounded directed conversion: no
                // native-float precheck and no intermediate FBig rounding, so
                // boundary results are exact and there is no double rounding.
                // The rational loses the sign of a zero result, supplied per IEEE.
                let exact: RBig = rbig_of::<$ty>(lhs) $operator rbig_of::<$ty>(rhs);
                let output =
                    round_rational::<$ty>(&exact, direction, ($zero_negative)(lhs, rhs, direction));
                if !output.is_finite() {
                    return Err(overflow_error());
                }
                Ok(Rounded::new(output, direction))
            }
        }
    };
}

/// Sign of an exact-zero sum/difference under IEEE 754: `+0` in every rounding
/// mode except `Down` (`-0`), unless both zero operands already share a sign.
fn add_zero_negative<T: Copy + PartialEq + SignBit>(a: T, b: T, direction: Direction) -> bool {
    let both_pos_zero = a.is_zero() && b.is_zero() && !a.sign_negative() && !b.sign_negative();
    let both_neg_zero = a.is_zero() && b.is_zero() && a.sign_negative() && b.sign_negative();
    match direction {
        Direction::Down => !both_pos_zero,
        _ => both_neg_zero,
    }
}

trait SignBit: Copy {
    fn is_zero(self) -> bool;
    fn sign_negative(self) -> bool;
    fn neg(self) -> Self;
}
impl SignBit for f64 {
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn sign_negative(self) -> bool {
        self.is_sign_negative()
    }
    fn neg(self) -> Self {
        -self
    }
}
impl SignBit for f32 {
    fn is_zero(self) -> bool {
        self == 0.0
    }
    fn sign_negative(self) -> bool {
        self.is_sign_negative()
    }
    fn neg(self) -> Self {
        -self
    }
}

macro_rules! directed_binary_all {
    ($ty:ty) => {
        directed_binary!($ty; Add, +, |_b: $ty| false,
            |a: $ty, b: $ty, d| add_zero_negative(a, b, d));
        directed_binary!($ty; Sub, -, |_b: $ty| false,
            |a: $ty, b: $ty, d| add_zero_negative(a, SignBit::neg(b), d));
        directed_binary!($ty; Mul, *, |_b: $ty| false,
            |a: $ty, b: $ty, _d| SignBit::sign_negative(a) ^ SignBit::sign_negative(b));
        // dashu 0.5 RBig division is correct; the zero quotient's sign is xor of signs.
        directed_binary!($ty; Div, /, |b: $ty| b == 0.0,
            |a: $ty, b: $ty, _d| SignBit::sign_negative(a) ^ SignBit::sign_negative(b));
    };
}

directed_binary_all!(f64);
directed_binary_all!(f32);

macro_rules! directed_powi {
    ($ty:ty) => {
        impl DirectedPowI<$ty, IBig> for Dashu {
            fn eval(base: $ty, exponent: &IBig, direction: Direction) -> Result<Rounded<$ty>> {
                if !base.is_finite() {
                    return Err(non_finite_input());
                }
                if base == 0.0 && exponent < &IBig::ZERO {
                    return Err(Error::new(
                        ErrorKind::DivisionByZero,
                        "zero to a negative power",
                    ));
                }
                let odd = exponent.clone().into_parts().1.bit(0);

                // A result that overflows to +/-inf is resolved from the cheap native
                // value: the arbitrary-precision powi would otherwise hand `fbig_rat`
                // an FBig infinity, which dashu-float panics on.
                if let Ok(small_exponent) = i32::try_from(exponent) {
                    let native = base.powi(small_exponent);
                    if !native.is_finite() {
                        return extreme_power::<$ty>(native.is_sign_negative(), true, direction);
                    }
                } else if base.abs() == 1.0 {
                    let output = if base.is_sign_negative() && odd {
                        -1.0
                    } else {
                        1.0
                    };
                    return Ok(Rounded::new(output, direction));
                } else if base != 0.0 {
                    // Outside i32, retain exact evaluation near one. Only classify a
                    // result structurally when it is separated from the primitive
                    // overflow/underflow boundary by a large logarithmic margin.
                    let exponent_f64 = i64::try_from(exponent).ok().map(|value| value as f64);
                    let magnitude_log2 = exponent_f64
                        .map(|value| value * base.abs().log2() as f64)
                        .unwrap_or_else(|| {
                            if (exponent > &IBig::ZERO) == (base.abs() > 1.0) {
                                f64::INFINITY
                            } else {
                                f64::NEG_INFINITY
                            }
                        });
                    if magnitude_log2 > 1200.0 || magnitude_log2 < -1200.0 {
                        return extreme_power::<$ty>(
                            base.is_sign_negative() && odd,
                            magnitude_log2.is_sign_positive(),
                            direction,
                        );
                    }
                }
                // A zero power result keeps base's sign only for an odd exponent.
                let zero_negative = base.is_sign_negative() && odd;
                eval_adaptive::<$ty>(direction, start_precision(0), zero_negative, |prec, dir| {
                    let raw = match dir {
                        Direction::Up => {
                            let b = FBig::<Up>::try_from(base)
                                .map_err(|_| non_finite_input())?
                                .with_precision(prec)
                                .value();
                            fbig_rat(&catch_backend(|| b.powi(exponent.clone()))?)
                        }
                        Direction::Down => {
                            let b = FBig::<Down>::try_from(base)
                                .map_err(|_| non_finite_input())?
                                .with_precision(prec)
                                .value();
                            fbig_rat(&catch_backend(|| b.powi(exponent.clone()))?)
                        }
                        Direction::Nearest => {
                            let b = FBig::<HalfEven>::try_from(base)
                                .map_err(|_| non_finite_input())?
                                .with_precision(prec)
                                .value();
                            fbig_rat(&catch_backend(|| b.powi(exponent.clone()))?)
                        }
                    };
                    Ok(raw)
                })
            }
        }

        impl DirectedPowI<$ty, i32> for Dashu {
            fn eval(base: $ty, exponent: &i32, direction: Direction) -> Result<Rounded<$ty>> {
                <Self as DirectedPowI<$ty, IBig>>::eval(base, &IBig::from(*exponent), direction)
            }
        }
    };
}

directed_powi!(f64);
directed_powi!(f32);

/// Exact rational of a finite primitive float.
fn rbig_of<T: PrimitiveRound>(value: T) -> RBig {
    value.to_rat().expect("input already checked finite")
}

macro_rules! convert_to_primitive {
    ($from:ty, $to:ty, $to_rat:expr) => {
        impl Convert<$from, $to> for Dashu {
            fn convert(value: &$from, direction: Direction) -> Result<Rounded<$to>> {
                // Exact-number conversion saturates to +/-inf on overflow, exactly
                // like MPFR's to_fXX_round; it does not raise an overflow error.
                let exact: RBig = ($to_rat)(value);
                Ok(Rounded::new(
                    round_rational::<$to>(&exact, direction, false),
                    direction,
                ))
            }
        }
    };
}

convert_to_primitive!(RBig, f64, |v: &RBig| v.clone());
convert_to_primitive!(RBig, f32, |v: &RBig| v.clone());
convert_to_primitive!(IBig, f64, |v: &IBig| RBig::from(v.clone()));
convert_to_primitive!(IBig, f32, |v: &IBig| RBig::from(v.clone()));
convert_to_primitive!(UBig, f64, |v: &UBig| RBig::from(v.clone()));
convert_to_primitive!(UBig, f32, |v: &UBig| RBig::from(v.clone()));
