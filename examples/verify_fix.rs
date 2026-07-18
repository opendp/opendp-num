//! Drives the opendp-num Dashu *adapter* for each adapter-bug finding's exact
//! operands and checks the correctly-rounded directed result. Confirms the
//! rounding fix in src/backend/dashu.rs.
//!
//! Run: cargo run --example verify_fix --features all-backends

use dashu::integer::IBig;
use opendp_num::{
    Add, Convert, DirectedBinary, DirectedPowI, DirectedUnary, Direction, Div, Exp, ExpM1, Ln1p,
    Mul, Sqrt, Sub, backend::dashu::Dashu,
};

fn check_bits(name: &str, got: u64, want: u64) {
    let ok = got == want;
    println!(
        "  [{}] {name}: got 0x{got:016x} want 0x{want:016x}",
        if ok { "PASS" } else { "FAIL" }
    );
    assert!(ok, "{name} mismatch");
}

fn main() {
    // DASHU-002: (f64::MAX) - 1.0, Down
    let r = <Dashu as DirectedBinary<Sub, f64>>::eval(f64::MAX, 1.0, Direction::Down).unwrap();
    check_bits("DASHU-002 sub Down", r.value.to_bits(), 0x7feffffffffffffe);

    // DASHU-010: 1.0 / tiny, Down -> f64::MAX (representable, not overflow)
    let r = <Dashu as DirectedBinary<Div, f64>>::eval(
        1.0,
        f64::from_bits(0x00000000003ff000),
        Direction::Down,
    )
    .unwrap();
    check_bits("DASHU-010 div Down", r.value.to_bits(), 0x7fefffffffffffff);

    // DASHU-010 (Up manifestation): -0.999.../tiny, Up -> -f64::MAX
    let r = <Dashu as DirectedBinary<Div, f64>>::eval(
        f64::from_bits(0xbfefffffffffffff),
        f64::from_bits(0x000000003ff00000),
        Direction::Up,
    )
    .unwrap();
    check_bits("DASHU-010 div Up", r.value.to_bits(), 0xffefffffffffffff);

    // DASHU-012: min_subnormal + (f64 just above -1), Up
    let r = <Dashu as DirectedBinary<Add, f64>>::eval(
        f64::from_bits(0x0000000000000001),
        f64::from_bits(0xbfefffffffffffff),
        Direction::Up,
    )
    .unwrap();
    check_bits("DASHU-012 add Up", r.value.to_bits(), 0xbfeffffffffffffe);

    // DASHU-016: directed div, Up
    let r = <Dashu as DirectedBinary<Div, f64>>::eval(
        f64::from_bits(0x0010000000070000),
        f64::from_bits(0x3ff00e0000010000),
        Direction::Up,
    )
    .unwrap();
    check_bits("DASHU-016 div Up", r.value.to_bits(), 0x000ff20c35575476);

    // DASHU-018: -0.0 + tiny, Up -> operand preserved
    let r = <Dashu as DirectedBinary<Add, f64>>::eval(
        f64::from_bits(0x8000000000000000),
        f64::from_bits(0x0000003ff0020000),
        Direction::Up,
    )
    .unwrap();
    check_bits("DASHU-018 add Up", r.value.to_bits(), 0x0000003ff0020000);

    // DASHU-019: min_subnormal * min_subnormal, Down -> +0
    let r = <Dashu as DirectedBinary<Mul, f64>>::eval(
        f64::from_bits(0x0000000000000001),
        f64::from_bits(0x0000000000000001),
        Direction::Down,
    )
    .unwrap();
    check_bits("DASHU-019 mul Down", r.value.to_bits(), 0x0000000000000000);

    // DASHU-006: ln1p(2^-1074), Down -> +0
    let r =
        <Dashu as DirectedUnary<Ln1p, f64>>::eval(f64::from_bits(1), Direction::Down).unwrap();
    check_bits("DASHU-006 ln1p Down", r.value.to_bits(), 0x0000000000000000);

    // DASHU-013: expm1(2^-1022), Up -> next f64 above min-normal
    let r = <Dashu as DirectedUnary<ExpM1, f64>>::eval(
        f64::from_bits(0x0010000000000000),
        Direction::Up,
    )
    .unwrap();
    check_bits("DASHU-013 expm1 Up", r.value.to_bits(), 0x0010000000000001);

    // DASHU-011: powi(f64::MAX, -53), Down -> +0 (positive underflow, never negative)
    let r = <Dashu as DirectedPowI<f64>>::eval(f64::MAX, -53, Direction::Down).unwrap();
    check_bits("DASHU-011 powi Down", r.value.to_bits(), 0x0000000000000000);

    // DASHU-015: (-(2^128 - 1)) -> f64, Up
    let n: IBig = (IBig::from(1) << 128) - IBig::from(1);
    let r = <Dashu as Convert<IBig, f64>>::convert(&(-n), Direction::Up).unwrap();
    check_bits("DASHU-015 int->f64 Up", r.value.to_bits(), 0xc7efffffffffffff);

    // DASHU-004 (f32): exp underflow. Up -> min positive subnormal; Down -> +0.
    let up = <Dashu as DirectedUnary<Exp, f32>>::eval(f32::from_bits(0xfefa39ef), Direction::Up)
        .unwrap();
    println!(
        "  [{}] DASHU-004 exp f32 Up: got 0x{:08x} want 0x00000001",
        if up.value.to_bits() == 0x00000001 { "PASS" } else { "FAIL" },
        up.value.to_bits()
    );
    assert_eq!(up.value.to_bits(), 0x00000001, "DASHU-004 Up");
    let down =
        <Dashu as DirectedUnary<Exp, f32>>::eval(f32::from_bits(0xd52d3051), Direction::Down)
            .unwrap();
    println!(
        "  [{}] DASHU-004 exp f32 Down: got 0x{:08x} want 0x00000000",
        if down.value.to_bits() == 0x00000000 { "PASS" } else { "FAIL" },
        down.value.to_bits()
    );
    assert_eq!(down.value.to_bits(), 0x00000000, "DASHU-004 Down");

    // Regression (found by re-fuzzing the rewrite): signed zero and overflow.

    // +0 + (-0), Down -> -0 (IEEE: cancellation sign is - only under Down).
    let r = <Dashu as DirectedBinary<Add, f64>>::eval(0.0, -0.0, Direction::Down).unwrap();
    check_bits("signed-zero add Down", r.value.to_bits(), 0x8000000000000000);
    // +0 + (-0), Up -> +0.
    let r = <Dashu as DirectedBinary<Add, f64>>::eval(0.0, -0.0, Direction::Up).unwrap();
    check_bits("signed-zero add Up", r.value.to_bits(), 0x0000000000000000);
    // (-0) * 5, Down -> -0 (sign = xor of operand signs).
    let r = <Dashu as DirectedBinary<Mul, f64>>::eval(-0.0, 5.0, Direction::Down).unwrap();
    check_bits("signed-zero mul", r.value.to_bits(), 0x8000000000000000);

    // Huge integer -> f32, Down -> -inf (conversion saturates, no error; matches MPFR).
    let big: IBig = "-10907481356194159294629842447337828624482641619962326924318"
        .parse()
        .unwrap();
    let r = <Dashu as Convert<IBig, f32>>::convert(&big, Direction::Down).unwrap();
    let ok = r.value == f32::NEG_INFINITY;
    println!(
        "  [{}] huge int->f32 Down: got {} want -inf",
        if ok { "PASS" } else { "FAIL" },
        r.value
    );
    assert!(ok, "huge int->f32 should saturate to -inf");

    // Nearest conversion sanity: 1/3 -> f64 nearest.
    let third = <Dashu as opendp_num::FromParts<_, IBig, dashu::integer::UBig>>::from_parts(
        IBig::from(1),
        dashu::integer::UBig::from(3u8),
    )
    .unwrap();
    let r = <Dashu as Convert<_, f64>>::convert(&third, Direction::Nearest).unwrap();
    check_bits("1/3 -> f64 Nearest", r.value.to_bits(), (1.0f64 / 3.0).to_bits());

    // Signed zero of sign-preserving transcendentals (found by continued fuzzing):
    // ln1p(-0), expm1(-0), sqrt(-0) all keep the negative zero.
    let r = <Dashu as DirectedUnary<Ln1p, f64>>::eval(-0.0, Direction::Up).unwrap();
    check_bits("ln1p(-0) Up", r.value.to_bits(), 0x8000000000000000);
    let r = <Dashu as DirectedUnary<ExpM1, f64>>::eval(-0.0, Direction::Up).unwrap();
    check_bits("expm1(-0) Up", r.value.to_bits(), 0x8000000000000000);
    let r = <Dashu as DirectedUnary<Sqrt, f64>>::eval(-0.0, Direction::Down).unwrap();
    check_bits("sqrt(-0) Down", r.value.to_bits(), 0x8000000000000000);

    // powi overflow must return a clean Overflow error / saturate, not panic on an
    // FBig infinity: 2^2000 far exceeds f64::MAX.
    let r = <Dashu as DirectedPowI<f64>>::eval(2.0, 2000, Direction::Down).unwrap();
    check_bits("powi overflow Down", r.value.to_bits(), f64::MAX.to_bits());
    assert!(
        <Dashu as DirectedPowI<f64>>::eval(2.0, 2000, Direction::Up).is_err(),
        "powi overflow Up should be an Overflow error"
    );
    println!("  [PASS] powi overflow Up: Overflow error");

    println!("\nAll adapter-fix checks passed.");
}
