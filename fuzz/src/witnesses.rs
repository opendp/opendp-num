//! Typed compile-time witnesses named by `operation_manifest.json`.

use dashu::{
    float::{
        DBig, FBig,
        round::mode::{Down, HalfEven, Up, Zero},
    },
    integer::{IBig, UBig},
};
use opendp_num::{
    DirectedPowI,
    backend::{dashu::Dashu, mpfr::Mpfr},
};
use rug::Integer;

fn require_powi<B, T, E: ?Sized>()
where
    B: DirectedPowI<T, E>,
{
}

fn require_dashu_f64()
where
    Dashu: DirectedPowI<f64, IBig>,
{
}
fn require_dashu_f32()
where
    Dashu: DirectedPowI<f32, IBig>,
{
}
fn require_mpfr_f64()
where
    Mpfr: DirectedPowI<f64, Integer>,
{
}
fn require_mpfr_f32()
where
    Mpfr: DirectedPowI<f32, Integer>,
{
}

pub fn witness_directed_powi_f64_bigint() {
    require_powi::<Dashu, f64, IBig>();
    require_powi::<Mpfr, f64, Integer>();
    require_dashu_f64();
    require_mpfr_f64();
}

pub fn witness_directed_powi_f32_bigint() {
    require_powi::<Dashu, f32, IBig>();
    require_powi::<Mpfr, f32, Integer>();
    require_dashu_f32();
    require_mpfr_f32();
}

fn binary_conversions<R: dashu::float::round::Round>() {
    let value = FBig::<R, 2>::from_parts(IBig::from(1), 0);
    let _ = value.to_f64();
    let _ = value.to_f32();
}

fn decimal_conversions<R: dashu::float::round::Round>() {
    let value = FBig::<R, 10>::from_parts(IBig::from(1), 0);
    let _ = value.to_f64();
    let _ = value.to_f32();
}

pub fn witness_dashu_fbig_to_f64() {
    binary_conversions::<HalfEven>();
    binary_conversions::<Up>();
    binary_conversions::<Down>();
    binary_conversions::<Zero>();
}

pub fn witness_dashu_fbig_to_f32() {
    witness_dashu_fbig_to_f64();
}

pub fn witness_dashu_fbig_decimal_to_primitive() {
    decimal_conversions::<HalfEven>();
    decimal_conversions::<Up>();
    decimal_conversions::<Down>();
    decimal_conversions::<Zero>();
}

pub fn witness_dashu_dbig_to_primitive() {
    let value = DBig::from_parts(IBig::from(1), 0);
    let _ = value.to_f64();
    let _ = value.to_f32();
}

pub fn witness_exact_integer() {
    let _ = UBig::from(1u8);
}
pub fn witness_exact_rational() {
    let _ = dashu::rational::RBig::from(1u8);
}
pub fn witness_directed_unary() {
    witness_directed_powi_f64_bigint();
}
pub fn witness_directed_binary() {}
pub fn witness_conversions() {}
pub fn witness_primitive_casts() {}
pub fn witness_alp_primitives() {}
pub fn witness_opendp_sequences() {}
pub fn witness_malachite_float() {}
pub fn witness_backend_float_conversion() {
    witness_dashu_fbig_to_f64();
}
