//! Direct dashu-API root-cause probe (not routed through opendp-num adapters).
//!
//! Classifies each flagship finding as an opendp-num *adapter* bug or a genuine
//! *dashu* bug by computing the correctly-rounded directed result from the exact
//! rational value, independent of dashu's `to_f64` approximation-sign reporting.
//!
//! Run: cargo run --example root_cause --features all-backends

use dashu::{
    base::Approximation,
    float::{
        FBig,
        round::{
            Rounding,
            mode::{Down, Up},
        },
    },
    integer::{IBig, UBig},
    rational::RBig,
};
use rug::{Float, float::Round};

const PREC53: usize = 53;

fn from_up_f64(v: FBig<Up>) -> f64 {
    match v.to_f64() {
        Approximation::Exact(x) | Approximation::Inexact(x, Rounding::AddOne) => x,
        Approximation::Inexact(x, _) => next_up_f64(x),
    }
}
fn from_down_f64(v: FBig<Down>) -> f64 {
    match v.to_f64() {
        Approximation::Exact(x) | Approximation::Inexact(x, Rounding::SubOne) => x,
        Approximation::Inexact(x, _) => next_down_f64(x),
    }
}

/// dashu-float directed unary at a chosen working precision, mirroring the adapter.
fn dashu_unary(x: f64, prec: usize, up: bool, op: &str) -> f64 {
    if up {
        let i = FBig::<Up>::try_from(x).unwrap().with_precision(prec).value();
        let r = match op {
            "ln1p" => i.ln_1p(),
            "expm1" => i.exp_m1(),
            "exp" => i.exp(),
            "ln" => i.ln(),
            _ => panic!("op"),
        };
        from_up_f64(r)
    } else {
        let i = FBig::<Down>::try_from(x).unwrap().with_precision(prec).value();
        let r = match op {
            "ln1p" => i.ln_1p(),
            "expm1" => i.exp_m1(),
            "exp" => i.exp(),
            "ln" => i.ln(),
            _ => panic!("op"),
        };
        from_down_f64(r)
    }
}

/// Exact rational value of a raw FBig (base-2 float is exactly rational),
/// bypassing `to_f64()` entirely — so it isolates transcendental vs conversion bugs.
fn fbig_rat<R: dashu::float::round::Round>(r: &FBig<R>) -> RBig {
    let repr = r.repr();
    let s = repr.significand().clone();
    let e = repr.exponent();
    if e >= 0 {
        RBig::from(s << (e as usize))
    } else {
        RBig::from_parts(s, UBig::from(1u8) << ((-e) as usize))
    }
}

fn dashu_powi(base: f64, exp: i32, prec: usize, up: bool) -> f64 {
    if up {
        let b = FBig::<Up>::try_from(base).unwrap().with_precision(prec).value();
        from_up_f64(b.powi(IBig::from(exp)))
    } else {
        let b = FBig::<Down>::try_from(base).unwrap().with_precision(prec).value();
        from_down_f64(b.powi(IBig::from(exp)))
    }
}

/// Exact f64 -> RBig, robust for subnormals (dashu's `try_from` rejects some).
fn f64_to_rbig(v: f64) -> RBig {
    assert!(v.is_finite());
    let b = v.to_bits();
    let sign = if b >> 63 == 1 { -1i8 } else { 1 };
    let exp = ((b >> 52) & 0x7ff) as i64;
    let mant = b & 0x000f_ffff_ffff_ffff;
    let (m, e) = if exp == 0 {
        (mant, -1074i64)
    } else {
        (mant | 0x0010_0000_0000_0000, exp - 1075)
    };
    let mut num = IBig::from(m) * IBig::from(sign);
    if e >= 0 {
        num <<= e as usize;
        RBig::from(num)
    } else {
        RBig::from_parts(num, UBig::from(1u8) << (-e) as usize)
    }
}

fn next_up_f64(v: f64) -> f64 {
    if v.is_nan() || v == f64::INFINITY {
        return v;
    }
    if v == 0.0 {
        return f64::from_bits(1);
    }
    let b = v.to_bits();
    f64::from_bits(if v > 0.0 { b + 1 } else { b - 1 })
}

fn next_down_f64(v: f64) -> f64 {
    if v.is_nan() || v == f64::NEG_INFINITY {
        return v;
    }
    if v == 0.0 {
        return f64::from_bits((1u64 << 63) | 1);
    }
    let b = v.to_bits();
    f64::from_bits(if v > 0.0 { b - 1 } else { b + 1 })
}

/// Correctly-rounded directed f64, decided purely by exact RBig comparison.
/// `try_from(f64)` is exact, so this never trusts dashu's `to_f64` sign.
fn round_directed(q: &RBig, up: bool) -> f64 {
    // A finite starting candidate; correctness comes from the exact-compare loop.
    let mut c = q.to_f64().value();
    if !c.is_finite() {
        // Clamp an overflowing seed back into range so the loop can settle.
        c = if c > 0.0 { f64::MAX } else { -f64::MAX };
    }
    let as_rat = |x: f64| f64_to_rbig(x);
    if up {
        // smallest representable f64 >= q (saturates at +f64::MAX)
        while as_rat(c) < *q {
            let n = next_up_f64(c);
            if !n.is_finite() {
                break;
            }
            c = n;
        }
        loop {
            let d = next_down_f64(c);
            if !d.is_finite() || as_rat(d) < *q {
                break;
            }
            c = d;
        }
    } else {
        // largest representable f64 <= q (saturates at -f64::MAX)
        while as_rat(c) > *q {
            let n = next_down_f64(c);
            if !n.is_finite() {
                break;
            }
            c = n;
        }
        loop {
            let u = next_up_f64(c);
            if !u.is_finite() || as_rat(u) > *q {
                break;
            }
            c = u;
        }
    }
    c
}

fn mpfr_round(q: &RBig, up: bool) -> f64 {
    let (num, den) = q.clone().into_parts();
    let n = rug::Integer::from_str_radix(&num.to_string(), 10).unwrap();
    let d = rug::Integer::from_str_radix(&den.to_string(), 10).unwrap();
    let dir = if up { Round::Up } else { Round::Down };
    let mut f = Float::with_val(2000, &n);
    f /= Float::with_val(2000, &d);
    let (val, _) = Float::with_val_round(53, &f, dir);
    val.to_f64()
}

fn bits(v: f64) -> String {
    format!("{v:.6e} (0x{:016x})", v.to_bits())
}

fn main() {
    println!("== DASHU-015: directed IBig -> f64 ==");
    // -(2^128 - 1), Up. Adapter returned dashu's value unchanged.
    let n: IBig = (IBig::from(1) << 128) - IBig::from(1);
    let neg = -n.clone();
    let approx = neg.to_f64(); // the exact call the adapter relies on
    let q = RBig::from(neg.clone());
    let correct_up = round_directed(&q, true);
    let mpfr_up = mpfr_round(&q, true);
    println!("  value               = -(2^128-1)");
    println!("  IBig::to_f64()      = {approx:?}");
    // Is the reported approximation sign truthful? sign(approx_value - exact).
    let approx_val = approx.value();
    let approx_rat = f64_to_rbig(approx_val);
    let true_err_sign = if approx_rat > q {
        "Positive (approx > exact)"
    } else if approx_rat < q {
        "Negative (approx < exact)"
    } else {
        "Zero (exact)"
    };
    println!("  actual sign(approx-exact) = {true_err_sign}");
    println!("  correct Up (exact/MPFR-indep) = {}", bits(correct_up));
    println!("  MPFR Up                       = {}", bits(mpfr_up));
    println!(
        "  => independent==MPFR: {}",
        correct_up.to_bits() == mpfr_up.to_bits()
    );

    println!("\n== DASHU-010: div overflow precheck (Down) 1 / tiny ==");
    let lhs = 1.0_f64;
    let rhs = f64::from_bits(0x00000000003ff000);
    let native = lhs / rhs;
    let q = f64_to_rbig(lhs) / f64_to_rbig(rhs);
    let correct_down = round_directed(&q, false);
    let mpfr_down = mpfr_round(&q, false);
    println!("  native f64 lhs/rhs  = {} (precheck sees this)", bits(native));
    println!("  correct Down (exact)= {}", bits(correct_down));
    println!("  MPFR Down           = {}", bits(mpfr_down));
    println!("  f64::MAX            = {}", bits(f64::MAX));
    println!(
        "  => result representable & ==MPFR: {}",
        correct_down.to_bits() == mpfr_down.to_bits()
    );

    println!("\n== DASHU-016: directed div (Up), one ULP too high ==");
    let lhs = f64::from_bits(0x0010000000070000);
    let rhs = f64::from_bits(0x3ff00e0000010000);
    let q = f64_to_rbig(lhs) / f64_to_rbig(rhs);
    let correct_up = round_directed(&q, true);
    let mpfr_up = mpfr_round(&q, true);
    println!("  exact-RBig-then-round Up = {}", bits(correct_up));
    println!("  MPFR Up                  = {}", bits(mpfr_up));
    println!(
        "  => exact-rational path ==MPFR: {}",
        correct_up.to_bits() == mpfr_up.to_bits()
    );

    // ---- Arithmetic cluster: does exact-RBig-then-single-round match MPFR? ----
    println!("\n== Arithmetic cluster (exact-RBig vs MPFR) ==");
    let arith = |name: &str, lb: u64, rb: u64, op: char, up: bool| {
        let l = f64_to_rbig(f64::from_bits(lb));
        let r = f64_to_rbig(f64::from_bits(rb));
        let q = match op {
            '+' => l + r,
            '-' => l - r,
            '*' => l * r,
            _ => unreachable!(),
        };
        let exact = round_directed(&q, up);
        let mpfr = mpfr_round(&q, up);
        println!(
            "  {name:9} {op} {:>4}: exact-RBig={}  MPFR={}  match={}",
            if up { "Up" } else { "Down" },
            bits(exact),
            bits(mpfr),
            exact.to_bits() == mpfr.to_bits()
        );
    };
    arith("DASHU-002", 0x7fefffffffffffff, 0x3ff0000000000000, '-', false);
    arith("DASHU-012", 0x0000000000000001, 0xbfefffffffffffff, '+', true);
    arith("DASHU-018", 0x8000000000000000, 0x0000003ff0020000, '+', true);
    arith("DASHU-019", 0x0000000000000001, 0x0000000000000001, '*', false);

    // ---- Transcendentals: does raising dashu working precision fix it? ----
    println!("\n== Transcendentals: dashu@53 vs dashu@200 (adapter uses 53) ==");
    let uni = |name: &str, xb: u64, up: bool, op: &str| {
        let x = f64::from_bits(xb);
        let lo = dashu_unary(x, PREC53, up, op);
        let hi = dashu_unary(x, 200, up, op);
        println!(
            "  {name:9} {op:5} {:>4}: dashu@53={}  dashu@200={}",
            if up { "Up" } else { "Down" },
            bits(lo),
            bits(hi)
        );
    };
    uni("DASHU-006", 0x0000000000000001, false, "ln1p"); // correct Down = +0
    uni("DASHU-013", 0x0010000000000000, true, "expm1"); // correct Up = next f64 above min-normal
    let p53 = dashu_powi(f64::from_bits(0x7fefffffffffffff), -53, PREC53, false);
    let p200 = dashu_powi(f64::from_bits(0x7fefffffffffffff), -53, 200, false);
    println!("  DASHU-011 powi  Down: dashu@53={}  dashu@200={} (correct=+0)", bits(p53), bits(p200));
    // DASHU-007 log2(min-normal)=-1022 exactly; adapter uses log2_bounds (loose).
    // Test a correctly-rounded route: ln(x)/ln(2) at high precision.
    let x = f64::from_bits(0x0010000000000000);
    let l53 = {
        let i = FBig::<Up>::try_from(x).unwrap().with_precision(PREC53).value();
        from_up_f64(i.ln() / FBig::<Up>::try_from(2.0f64).unwrap().with_precision(PREC53).value().ln())
    };
    let l200 = {
        let i = FBig::<Up>::try_from(x).unwrap().with_precision(200).value();
        from_up_f64(i.ln() / FBig::<Up>::try_from(2.0f64).unwrap().with_precision(200).value().ln())
    };
    println!("  DASHU-007 log2  Up  : ln/ln2 @53={}  @200={} (correct=-1022)", bits(l53), bits(l200));

    // ---- Discriminator: is the RAW FBig@200 result correct BEFORE to_f64? ----
    // Compares the exact rational of the raw dashu-float result to the boundary,
    // bypassing the unreliable to_f64() proven wrong in DASHU-015.
    println!("\n== Transcendental discriminator (raw FBig@200, pre-to_f64) ==");
    // DASHU-006: ln1p(2^-1074). True value < 2^-1074, so correct Down = +0.
    {
        let x = f64::from_bits(0x0000000000000001);
        let raw = FBig::<Down>::try_from(x).unwrap().with_precision(4096).value().ln_1p();
        let r = fbig_rat(&raw);
        let thr = f64_to_rbig(x);
        println!(
            "  DASHU-006 ln1p : raw {} 2^-1074  => {}",
            if r < thr { "<" } else if r > thr { ">" } else { "==" },
            if r < thr { "FBig correct (bug is to_f64/conversion)" } else { "FBig ln_1p wrong (genuine transcendental)" }
        );
    }
    // DASHU-013: expm1(2^-1022) Up. True value > 2^-1022.
    {
        let x = f64::from_bits(0x0010000000000000);
        let raw = FBig::<Up>::try_from(x).unwrap().with_precision(4096).value().exp_m1();
        let r = fbig_rat(&raw);
        let thr = f64_to_rbig(x);
        println!(
            "  DASHU-013 expm1: raw {} 2^-1022  => {}",
            if r < thr { "<" } else if r > thr { ">" } else { "==" },
            if r > thr { "FBig correct (bug is to_f64/conversion)" } else { "FBig exp_m1 wrong (genuine transcendental)" }
        );
    }
    // DASHU-011: powi(MAX,-53) Down. True value > 0.
    {
        let raw = FBig::<Down>::try_from(f64::from_bits(0x7fefffffffffffff)).unwrap()
            .with_precision(4096).value().powi(IBig::from(-53));
        let r = fbig_rat(&raw);
        let zero = RBig::from(IBig::from(0));
        println!(
            "  DASHU-011 powi : raw sign {}  => {}",
            if r < zero { "NEGATIVE" } else if r > zero { "positive" } else { "zero" },
            if r < zero { "FBig powi wrong sign (genuine transcendental)" } else { "FBig sign ok (bug is to_f64/conversion)" }
        );
        // Attribute the conversion fault: what does dashu's to_f64 report for this
        // tiny positive value, and what does the adapter's from_down do with it?
        println!("    raw.to_f64() = {:?}", raw.to_f64());
        println!("    adapter from_down(raw) = {}", bits(from_down_f64(raw)));
        println!("    next_down_f64(0.0)    = {} (adapter helper)", bits(next_down_f64(0.0)));
    }
}
