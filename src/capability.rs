use crate::{Direction, Result, Rounded};

/// Associates a provider with its native exact-number types.
pub trait Backend {
    type Natural;
    type Integer;
    type Rational;
}

/// Exact binary operation evaluated into the provider's native output type.
pub trait ExactBinary<Op, T> {
    fn eval(lhs: &T, rhs: &T) -> T;
}

/// Exact unary operation evaluated into the provider's native output type.
pub trait ExactUnary<Op, T> {
    fn eval(value: &T) -> T;
}

/// Checked operation whose mathematical preconditions may be violated.
pub trait CheckedBinary<Op, T> {
    fn eval(lhs: &T, rhs: &T) -> Result<T>;
}

/// Correctly directed primitive unary operation.
pub trait DirectedUnary<Op, T> {
    fn eval(value: T, direction: Direction) -> Result<Rounded<T>>;
}

/// Correctly directed primitive binary operation.
pub trait DirectedBinary<Op, T> {
    fn eval(lhs: T, rhs: T, direction: Direction) -> Result<Rounded<T>>;
}

/// Correctly directed primitive power with a signed integer exponent.
///
/// `E` is provider-native so the contract does not silently narrow an
/// arbitrary-precision exponent before the backend sees it.
pub trait DirectedPowI<T, E: ?Sized = i32> {
    fn eval(base: T, exponent: &E, direction: Direction) -> Result<Rounded<T>>;
}

/// Conversion with an explicit rounding contract.
pub trait Convert<From, To> {
    fn convert(value: &From, direction: Direction) -> Result<Rounded<To>>;
}

/// Construct a canonical rational from native provider parts.
pub trait FromParts<Rational, Integer, Natural> {
    fn from_parts(numerator: Integer, denominator: Natural) -> Result<Rational>;
}

/// Consume a canonical rational into native provider parts.
pub trait IntoParts<Rational, Integer, Natural> {
    fn into_parts(value: Rational) -> (Integer, Natural);
}
