#![no_main]

use std::str::FromStr;

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
use opendp_num::{Convert, Direction, FromParts, IntoParts, Result, Rounded};
use opendp_num_fuzz::{
    ConversionCase, any_direction, fail, signed_decimal, special_f32, special_f64, split_evenly,
    unsigned_decimal,
};
use rug::{Float, float::Round};

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(case) = ConversionCase::arbitrary(&mut unstructured) else {
        return;
    };
    let direction = any_direction(case.direction);
    let chunks = split_evenly(&case.payload, 2);
    let signed = signed_decimal(chunks[0], case.sign, case.selector);
    let unsigned = unsigned_decimal(chunks[0], case.selector);
    let mut denominator = unsigned_decimal(chunks[1], case.selector.wrapping_add(1));
    if denominator == "0" {
        denominator = "1".to_owned();
    }

    match case.operation % 9 {
        0 => {
            let (dashu, rug) = rationals(&signed, &denominator);
            compare_f64(
                "rational_to_f64",
                data,
                &[
                    ("numerator", signed),
                    ("denominator", denominator),
                    ("direction", format!("{direction:?}")),
                ],
                <Dashu as Convert<_, f64>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f64>>::convert(&rug, direction),
            );
        }
        1 => {
            let (dashu, rug) = rationals(&signed, &denominator);
            compare_f32(
                "rational_to_f32",
                data,
                &[
                    ("numerator", signed),
                    ("denominator", denominator),
                    ("direction", format!("{direction:?}")),
                ],
                <Dashu as Convert<_, f32>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f32>>::convert(&rug, direction),
            );
        }
        2 => {
            let dashu = dashu::integer::IBig::from_str(&signed).unwrap();
            let rug = rug::Integer::from_str(&signed).unwrap();
            compare_f64(
                "integer_to_f64",
                data,
                &[("integer", signed), ("direction", format!("{direction:?}"))],
                <Dashu as Convert<_, f64>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f64>>::convert(&rug, direction),
            );
        }
        3 => {
            let dashu = dashu::integer::IBig::from_str(&signed).unwrap();
            let rug = rug::Integer::from_str(&signed).unwrap();
            compare_f32(
                "integer_to_f32",
                data,
                &[("integer", signed), ("direction", format!("{direction:?}"))],
                <Dashu as Convert<_, f32>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f32>>::convert(&rug, direction),
            );
        }
        4 => {
            let dashu = dashu::integer::UBig::from_str(&unsigned).unwrap();
            let rug = rug::Integer::from_str(&unsigned).unwrap();
            compare_f64(
                "natural_to_f64",
                data,
                &[
                    ("natural", unsigned),
                    ("direction", format!("{direction:?}")),
                ],
                <Dashu as Convert<_, f64>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f64>>::convert(&rug, direction),
            );
        }
        5 => {
            let dashu = dashu::integer::UBig::from_str(&unsigned).unwrap();
            let rug = rug::Integer::from_str(&unsigned).unwrap();
            compare_f32(
                "natural_to_f32",
                data,
                &[
                    ("natural", unsigned),
                    ("direction", format!("{direction:?}")),
                ],
                <Dashu as Convert<_, f32>>::convert(&dashu, direction),
                <Mpfr as Convert<_, f32>>::convert(&rug, direction),
            );
        }
        6 => {
            let value = special_f64(case.selector, case.bits);
            check_float_to_rational_f64(value, data);
        }
        7 => {
            let value = special_f32(case.selector, case.bits);
            check_float_to_rational_f32(value, data);
        }
        _ => {
            let value = special_f64(case.selector, case.bits);
            check_f64_to_f32(value, direction, data);
        }
    }
});

fn rationals(numerator: &str, denominator: &str) -> (dashu::rational::RBig, rug::Rational) {
    let dashu = <Dashu as FromParts<
        dashu::rational::RBig,
        dashu::integer::IBig,
        dashu::integer::UBig,
    >>::from_parts(
        dashu::integer::IBig::from_str(numerator).unwrap(),
        dashu::integer::UBig::from_str(denominator).unwrap(),
    )
    .unwrap();
    let rug = <Mpfr as FromParts<rug::Rational, rug::Integer, rug::Integer>>::from_parts(
        rug::Integer::from_str(numerator).unwrap(),
        rug::Integer::from_str(denominator).unwrap(),
    )
    .unwrap();
    (dashu, rug)
}

fn compare_f64(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    dashu: Result<Rounded<f64>>,
    mpfr: Result<Rounded<f64>>,
) {
    match (dashu, mpfr) {
        (Ok(dashu), Ok(mpfr)) if dashu.value.to_bits() == mpfr.value.to_bits() => {}
        (dashu, mpfr) => fail(
            "conversions",
            operation,
            "directed conversion differs from MPFR",
            input,
            &[
                fields,
                &[
                    ("dashu", format!("{dashu:?}")),
                    ("mpfr", format!("{mpfr:?}")),
                ],
            ]
            .concat(),
        ),
    }
}

fn compare_f32(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    dashu: Result<Rounded<f32>>,
    mpfr: Result<Rounded<f32>>,
) {
    match (dashu, mpfr) {
        (Ok(dashu), Ok(mpfr)) if dashu.value.to_bits() == mpfr.value.to_bits() => {}
        (dashu, mpfr) => fail(
            "conversions",
            operation,
            "directed conversion differs from MPFR",
            input,
            &[
                fields,
                &[
                    ("dashu", format!("{dashu:?}")),
                    ("mpfr", format!("{mpfr:?}")),
                ],
            ]
            .concat(),
        ),
    }
}

fn check_float_to_rational_f64(value: f64, input: &[u8]) {
    let dashu = dashu::rational::RBig::try_from(value);
    let rug = rug::Rational::from_f64(value);
    match (dashu, rug) {
        (Ok(dashu), Some(rug)) => {
            let dashu = dashu_key(dashu);
            let rug = rug_key(rug);
            if dashu != rug {
                fail(
                    "conversions",
                    "f64_to_rational",
                    "exact float-to-rational conversion differs",
                    input,
                    &[
                        ("value", value.to_string()),
                        ("bits", format!("{:#018x}", value.to_bits())),
                        ("dashu", dashu),
                        ("mpfr", rug),
                    ],
                );
            }
        }
        (Err(_), None) => {}
        (dashu, rug) => fail(
            "conversions",
            "f64_to_rational",
            "finite/non-finite acceptance differs",
            input,
            &[
                ("value", value.to_string()),
                ("bits", format!("{:#018x}", value.to_bits())),
                ("dashu", format!("{dashu:?}")),
                ("mpfr", format!("{rug:?}")),
            ],
        ),
    }
}

fn check_float_to_rational_f32(value: f32, input: &[u8]) {
    let dashu = dashu::rational::RBig::try_from(value);
    let rug = rug::Rational::from_f32(value);
    match (dashu, rug) {
        (Ok(dashu), Some(rug)) => {
            let dashu = dashu_key(dashu);
            let rug = rug_key(rug);
            if dashu != rug {
                fail(
                    "conversions",
                    "f32_to_rational",
                    "exact float-to-rational conversion differs",
                    input,
                    &[
                        ("value", value.to_string()),
                        ("bits", format!("{:#010x}", value.to_bits())),
                        ("dashu", dashu),
                        ("mpfr", rug),
                    ],
                );
            }
        }
        (Err(_), None) => {}
        (dashu, rug) => fail(
            "conversions",
            "f32_to_rational",
            "finite/non-finite acceptance differs",
            input,
            &[
                ("value", value.to_string()),
                ("bits", format!("{:#010x}", value.to_bits())),
                ("dashu", format!("{dashu:?}")),
                ("mpfr", format!("{rug:?}")),
            ],
        ),
    }
}

fn check_f64_to_f32(value: f64, direction: Direction, input: &[u8]) {
    let actual = directed_f64_to_f32(value, direction);
    let round = match direction {
        Direction::Down => Round::Down,
        Direction::Nearest => Round::Nearest,
        Direction::Up => Round::Up,
    };
    let expected = Float::with_val(53, value).to_f32_round(round);
    if !(actual.is_nan() && expected.is_nan()) && actual.to_bits() != expected.to_bits() {
        fail(
            "conversions",
            "f64_to_f32",
            "primitive directed cast differs from MPFR",
            input,
            &[
                ("value", value.to_string()),
                ("value_bits", format!("{:#018x}", value.to_bits())),
                ("direction", format!("{direction:?}")),
                ("actual", format!("{} ({:#010x})", actual, actual.to_bits())),
                (
                    "mpfr",
                    format!("{} ({:#010x})", expected, expected.to_bits()),
                ),
            ],
        );
    }
}

fn directed_f64_to_f32(value: f64, direction: Direction) -> f32 {
    if value.is_nan() {
        return f32::NAN;
    }
    let nearest = value as f32;
    match direction {
        Direction::Nearest => nearest,
        Direction::Up if value > nearest as f64 => next_up_f32(nearest),
        Direction::Down if value < nearest as f64 => next_down_f32(nearest),
        _ => nearest,
    }
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
        return f32::from_bits(0x8000_0001);
    }
    let bits = value.to_bits();
    f32::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn dashu_key(value: dashu::rational::RBig) -> String {
    let (numerator, denominator) = <Dashu as IntoParts<
        dashu::rational::RBig,
        dashu::integer::IBig,
        dashu::integer::UBig,
    >>::into_parts(value);
    format!("{numerator}/{denominator}")
}

fn rug_key(value: rug::Rational) -> String {
    let (numerator, denominator) =
        <Mpfr as IntoParts<rug::Rational, rug::Integer, rug::Integer>>::into_parts(value);
    format!("{numerator}/{denominator}")
}
