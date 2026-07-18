#![no_main]

//! Differential fuzzing of the malachite-float directed backend against the
//! MPFR oracle: correctly rounded add/sub/mul/div, ln/ln1p/log2/exp/expm1/sqrt,
//! signed-integer power, and exact-number-to-primitive conversion, for f32 and
//! f64 in every directed rounding mode.

use std::str::FromStr;

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use malachite::{Integer, Natural, Rational};
use opendp_num::backend::{malachite::Malachite, mpfr::Mpfr};
use opendp_num::{
    Add, Convert, DirectedBinary, DirectedPowI, DirectedUnary, Direction, Div, Error, Exp, ExpM1,
    FromParts, Ln, Ln1p, Log2, Mul, Result, Rounded, Sqrt, Sub,
};
use opendp_num_fuzz::{
    BinaryCase, ConversionCase, UnaryCase, any_direction, catch_backend, fail, signed_decimal,
    special_f32, special_f64, split_evenly, unsigned_decimal,
};

const TARGET: &str = "malachite_float";

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(kind) = u8::arbitrary(&mut u) else {
        return;
    };
    match kind % 4 {
        0 => {
            if let Ok(case) = BinaryCase::arbitrary(&mut u) {
                binary(&case, data);
            }
        }
        1 => {
            if let Ok(case) = UnaryCase::arbitrary(&mut u) {
                unary(&case, data);
            }
        }
        2 => {
            if let Ok(case) = UnaryCase::arbitrary(&mut u) {
                powi(&case, data);
            }
        }
        _ => {
            if let Ok(case) = ConversionCase::arbitrary(&mut u) {
                conversion(&case, data);
            }
        }
    }
});

fn conversion(case: &ConversionCase, data: &[u8]) {
    let direction = any_direction(case.direction);
    let chunks = split_evenly(&case.payload, 2);
    let signed = signed_decimal(chunks[0], case.sign, case.selector);
    let unsigned = unsigned_decimal(chunks[0], case.selector);
    let mut den = unsigned_decimal(chunks[1], case.selector.wrapping_add(1));
    if den == "0" {
        den = "1".to_owned();
    }
    let f64_out = case.bits & 1 == 0;
    match case.operation % 3 {
        0 => {
            let mal = Integer::from_str(&signed).unwrap();
            let rug = rug::Integer::from_str(&signed).unwrap();
            let f = vec![("integer", signed.clone()), ("direction", format!("{direction:?}"))];
            if f64_out {
                compare64(
                    "integer_to_f64",
                    data,
                    &f,
                    <Malachite as Convert<_, f64>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f64>>::convert(&rug, direction),
                );
            } else {
                compare32(
                    "integer_to_f32",
                    data,
                    &f,
                    <Malachite as Convert<_, f32>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f32>>::convert(&rug, direction),
                );
            }
        }
        1 => {
            let mal = Natural::from_str(&unsigned).unwrap();
            let rug = rug::Integer::from_str(&unsigned).unwrap();
            let f = vec![("natural", unsigned.clone()), ("direction", format!("{direction:?}"))];
            if f64_out {
                compare64(
                    "natural_to_f64",
                    data,
                    &f,
                    <Malachite as Convert<_, f64>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f64>>::convert(&rug, direction),
                );
            } else {
                compare32(
                    "natural_to_f32",
                    data,
                    &f,
                    <Malachite as Convert<_, f32>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f32>>::convert(&rug, direction),
                );
            }
        }
        _ => {
            let mal = Rational::from_integers(
                Integer::from_str(&signed).unwrap(),
                Integer::from_str(&den).unwrap(),
            );
            let rug = <Mpfr as FromParts<rug::Rational, rug::Integer, rug::Integer>>::from_parts(
                rug::Integer::from_str(&signed).unwrap(),
                rug::Integer::from_str(&den).unwrap(),
            )
            .unwrap();
            let f = vec![
                ("numerator", signed.clone()),
                ("denominator", den.clone()),
                ("direction", format!("{direction:?}")),
            ];
            if f64_out {
                compare64(
                    "rational_to_f64",
                    data,
                    &f,
                    <Malachite as Convert<_, f64>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f64>>::convert(&rug, direction),
                );
            } else {
                compare32(
                    "rational_to_f32",
                    data,
                    &f,
                    <Malachite as Convert<_, f32>>::convert(&mal, direction),
                    <Mpfr as Convert<_, f32>>::convert(&rug, direction),
                );
            }
        }
    }
}


fn binary(case: &BinaryCase, data: &[u8]) {
    let direction = any_direction(case.direction);
    if case.format & 1 == 0 {
        let lhs = special_f64(case.lhs_selector, case.lhs_bits);
        let rhs = special_f64(case.rhs_selector, case.rhs_bits);
        let f = fields64(&[("lhs", lhs), ("rhs", rhs)], direction);
        match case.operation % 4 {
            0 => bin64::<Add>("add", lhs, rhs, direction, data, &f),
            1 => bin64::<Sub>("sub", lhs, rhs, direction, data, &f),
            2 => bin64::<Mul>("mul", lhs, rhs, direction, data, &f),
            _ => bin64::<Div>("div", lhs, rhs, direction, data, &f),
        }
    } else {
        let lhs = special_f32(case.lhs_selector, case.lhs_bits);
        let rhs = special_f32(case.rhs_selector, case.rhs_bits);
        let f = fields32(&[("lhs", lhs), ("rhs", rhs)], direction);
        match case.operation % 4 {
            0 => bin32::<Add>("add", lhs, rhs, direction, data, &f),
            1 => bin32::<Sub>("sub", lhs, rhs, direction, data, &f),
            2 => bin32::<Mul>("mul", lhs, rhs, direction, data, &f),
            _ => bin32::<Div>("div", lhs, rhs, direction, data, &f),
        }
    }
}

fn unary(case: &UnaryCase, data: &[u8]) {
    let direction = any_direction(case.direction);
    if case.format & 1 == 0 {
        let x = special_f64(case.selector, case.bits);
        let f = fields64(&[("value", x)], direction);
        match case.operation % 6 {
            0 => un64::<Ln>("ln", x, direction, data, &f),
            1 => un64::<Ln1p>("ln1p", x, direction, data, &f),
            2 => un64::<Log2>("log2", x, direction, data, &f),
            3 => un64::<Exp>("exp", x, direction, data, &f),
            4 => un64::<ExpM1>("expm1", x, direction, data, &f),
            _ => un64::<Sqrt>("sqrt", x, direction, data, &f),
        }
    } else {
        let x = special_f32(case.selector, case.bits);
        let f = fields32(&[("value", x)], direction);
        match case.operation % 6 {
            0 => un32::<Ln>("ln", x, direction, data, &f),
            1 => un32::<Ln1p>("ln1p", x, direction, data, &f),
            2 => un32::<Log2>("log2", x, direction, data, &f),
            3 => un32::<Exp>("exp", x, direction, data, &f),
            4 => un32::<ExpM1>("expm1", x, direction, data, &f),
            _ => un32::<Sqrt>("sqrt", x, direction, data, &f),
        }
    }
}

fn powi(case: &UnaryCase, data: &[u8]) {
    let direction = any_direction(case.direction);
    // Keep the exponent small so both backends stay affordable.
    let exponent = (case.exponent % 512) as i32;
    if case.format & 1 == 0 {
        let base = special_f64(case.selector, case.bits);
        let mut f = fields64(&[("base", base)], direction);
        f.push(("exponent", exponent.to_string()));
        let mal = catch_backend(TARGET, "powi", data, &f, || {
            <Malachite as DirectedPowI<f64>>::eval(base, exponent, direction)
        });
        let mpfr = catch_backend(TARGET, "powi", data, &f, || {
            <Mpfr as DirectedPowI<f64>>::eval(base, exponent, direction)
        });
        compare64("powi", data, &f, mal, mpfr);
    } else {
        let base = special_f32(case.selector, case.bits);
        let mut f = fields32(&[("base", base)], direction);
        f.push(("exponent", exponent.to_string()));
        let mal = catch_backend(TARGET, "powi", data, &f, || {
            <Malachite as DirectedPowI<f32>>::eval(base, exponent, direction)
        });
        let mpfr = catch_backend(TARGET, "powi", data, &f, || {
            <Mpfr as DirectedPowI<f32>>::eval(base, exponent, direction)
        });
        compare32("powi", data, &f, mal, mpfr);
    }
}

fn bin64<Op>(op: &str, lhs: f64, rhs: f64, d: Direction, data: &[u8], f: &[(&str, String)])
where
    Malachite: DirectedBinary<Op, f64>,
    Mpfr: DirectedBinary<Op, f64>,
{
    let mal = catch_backend(TARGET, op, data, f, || {
        <Malachite as DirectedBinary<Op, f64>>::eval(lhs, rhs, d)
    });
    let mpfr = catch_backend(TARGET, op, data, f, || {
        <Mpfr as DirectedBinary<Op, f64>>::eval(lhs, rhs, d)
    });
    compare64(op, data, f, mal, mpfr);
}

fn bin32<Op>(op: &str, lhs: f32, rhs: f32, d: Direction, data: &[u8], f: &[(&str, String)])
where
    Malachite: DirectedBinary<Op, f32>,
    Mpfr: DirectedBinary<Op, f32>,
{
    let mal = catch_backend(TARGET, op, data, f, || {
        <Malachite as DirectedBinary<Op, f32>>::eval(lhs, rhs, d)
    });
    let mpfr = catch_backend(TARGET, op, data, f, || {
        <Mpfr as DirectedBinary<Op, f32>>::eval(lhs, rhs, d)
    });
    compare32(op, data, f, mal, mpfr);
}

fn un64<Op>(op: &str, x: f64, d: Direction, data: &[u8], f: &[(&str, String)])
where
    Malachite: DirectedUnary<Op, f64>,
    Mpfr: DirectedUnary<Op, f64>,
{
    let mal = catch_backend(TARGET, op, data, f, || {
        <Malachite as DirectedUnary<Op, f64>>::eval(x, d)
    });
    let mpfr =
        catch_backend(TARGET, op, data, f, || <Mpfr as DirectedUnary<Op, f64>>::eval(x, d));
    compare64(op, data, f, mal, mpfr);
}

fn un32<Op>(op: &str, x: f32, d: Direction, data: &[u8], f: &[(&str, String)])
where
    Malachite: DirectedUnary<Op, f32>,
    Mpfr: DirectedUnary<Op, f32>,
{
    let mal = catch_backend(TARGET, op, data, f, || {
        <Malachite as DirectedUnary<Op, f32>>::eval(x, d)
    });
    let mpfr =
        catch_backend(TARGET, op, data, f, || <Mpfr as DirectedUnary<Op, f32>>::eval(x, d));
    compare32(op, data, f, mal, mpfr);
}

fn compare64(op: &str, data: &[u8], f: &[(&str, String)], mal: Result<Rounded<f64>>, mpfr: Result<Rounded<f64>>) {
    match (&mal, &mpfr) {
        (Ok(a), Ok(b)) if a.value.to_bits() == b.value.to_bits() => {}
        (Err(a), Err(b)) if a.kind == b.kind => {}
        _ => fail(
            TARGET,
            op,
            reason(&mal, &mpfr),
            data,
            &[f, &[("malachite", out64(&mal)), ("mpfr", out64(&mpfr))]].concat(),
        ),
    }
}

fn compare32(op: &str, data: &[u8], f: &[(&str, String)], mal: Result<Rounded<f32>>, mpfr: Result<Rounded<f32>>) {
    match (&mal, &mpfr) {
        (Ok(a), Ok(b)) if a.value.to_bits() == b.value.to_bits() => {}
        (Err(a), Err(b)) if a.kind == b.kind => {}
        _ => fail(
            TARGET,
            op,
            reason(&mal, &mpfr),
            data,
            &[f, &[("malachite", out32(&mal)), ("mpfr", out32(&mpfr))]].concat(),
        ),
    }
}

fn reason<T>(mal: &Result<T>, mpfr: &Result<T>) -> &'static str {
    match (mal, mpfr) {
        (Ok(_), Ok(_)) => "correctly rounded value differs from MPFR",
        (Ok(_), Err(_)) => "Malachite returned a value where MPFR reports an error",
        (Err(_), Ok(_)) => "Malachite returned an error for a valid MPFR result",
        (Err(_), Err(_)) => "Malachite and MPFR classify the error differently",
    }
}

fn out64(v: &Result<Rounded<f64>>) -> String {
    match v {
        Ok(v) => format!("{} ({:#018x})", v.value, v.value.to_bits()),
        Err(e) => err(e),
    }
}
fn out32(v: &Result<Rounded<f32>>) -> String {
    match v {
        Ok(v) => format!("{} ({:#010x})", v.value, v.value.to_bits()),
        Err(e) => err(e),
    }
}
fn err(e: &Error) -> String {
    format!("error {:?}: {}", e.kind, e.message)
}

fn fields64(vals: &[(&str, f64)], d: Direction) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    f.push(("format", "f64".to_owned()));
    for (name, v) in vals {
        let name: &'static str = match *name {
            "lhs" => "lhs",
            "rhs" => "rhs",
            "value" => "value",
            "base" => "base",
            _ => "arg",
        };
        f.push((name, format!("{v} ({:#018x})", v.to_bits())));
    }
    f.push(("direction", format!("{d:?}")));
    f
}

fn fields32(vals: &[(&str, f32)], d: Direction) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    f.push(("format", "f32".to_owned()));
    for (name, v) in vals {
        let name: &'static str = match *name {
            "lhs" => "lhs",
            "rhs" => "rhs",
            "value" => "value",
            "base" => "base",
            _ => "arg",
        };
        f.push((name, format!("{v} ({:#010x})", v.to_bits())));
    }
    f.push(("direction", format!("{d:?}")));
    f
}
