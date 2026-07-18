//! Backend-neutral numerical capability contracts for OpenDP.
//!
//! The crate deliberately does not invent wrapper number types. Instead it:
//! - names backend families with associated native types;
//! - blanket-implements exact operations from Rust's borrowed operators; and
//! - exposes individually implementable capabilities for operations whose
//!   semantics differ between providers, especially directed rounding.

#![forbid(unsafe_code)]

mod capability;
mod error;
mod operation;
mod rounding;

pub mod backend;
pub mod harness;

pub use capability::{
    Backend, CheckedBinary, Convert, DirectedBinary, DirectedPowI, DirectedUnary, ExactBinary,
    ExactUnary, FromParts, IntoParts,
};
pub use error::{Error, ErrorKind, Result};
pub use operation::{Add, Div, Exp, ExpM1, Ln, Ln1p, Log2, Mul, Neg, PowI, Rem, Sqrt, Sub};
pub use rounding::{Direction, Rounded};
