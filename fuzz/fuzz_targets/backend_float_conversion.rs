#![no_main]

use std::time::Instant;

use arbitrary::{Arbitrary, Unstructured};
use dashu::{
    base::{Approximation, Sign},
    float::{
        DBig, FBig,
        round::{
            Rounding,
            mode::{Down, HalfEven, Up, Zero},
        },
    },
    integer::{IBig, UBig},
};
use libfuzzer_sys::fuzz_target;
use opendp_num_fuzz::{catch_backend, fail};
use rug::{Float, Integer, Rational, float::Round, integer::Order};

const TARGET: &str = "backend_float_conversion";
const MAX_SIGNIFICAND_BYTES: usize = 512;
const MAX_EXPONENT: i32 = 20_000;

#[derive(Arbitrary, Debug)]
struct BigFloatConversionCase {
    format: u8,
    radix: u8,
    rounding: u8,
    negative: bool,
    exponent: i32,
    significand: Vec<u8>,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(case) = BigFloatConversionCase::arbitrary(&mut unstructured) else {
        return;
    };
    let magnitude_bytes = &case.significand[..case.significand.len().min(MAX_SIGNIFICAND_BYTES)];
    let magnitude = UBig::from_be_bytes(magnitude_bytes);
    let sign = if case.negative {
        Sign::Negative
    } else {
        Sign::Positive
    };
    let significand = IBig::from_parts(sign, magnitude);
    let exponent = case.exponent.clamp(-MAX_EXPONENT, MAX_EXPONENT) as isize;
    let radix = if case.radix & 1 == 0 { 2 } else { 10 };
    let exact = exact_rational(magnitude_bytes, case.negative, radix, exponent as i32);
    let format = if case.format & 1 == 0 { "f64" } else { "f32" };
    let fields = vec![
        ("contract", "backend_conformance".to_owned()),
        ("provider", "dashu".to_owned()),
        ("construction", "from_parts".to_owned()),
        ("source_type", format!("FBig<_,{radix}>")),
        ("source_precision", "0".to_owned()),
        ("significand_bits", (magnitude_bytes.len() * 8).to_string()),
        ("exponent", exponent.to_string()),
        ("format", format.to_owned()),
        ("oracle", "exact_rational".to_owned()),
        ("masked_by_adapter", "true".to_owned()),
        (
            "adapter_result",
            "not_exposed: opendp-num has no FBig-to-primitive capability".to_owned(),
        ),
        ("owner", "backend".to_owned()),
    ];

    macro_rules! probe {
        ($mode:ty, $base:literal, $mode_name:literal, $round:expr) => {{
            let started = Instant::now();
            if case.format & 1 == 0 {
                let observed = catch_backend(TARGET, "to_f64", data, &fields, || {
                    FBig::<$mode, $base>::from_parts(significand.clone(), exponent).to_f64()
                });
                check_f64(
                    data, &fields, $mode_name, $round, false, &exact, observed, started,
                );
            } else {
                let observed = catch_backend(TARGET, "to_f32", data, &fields, || {
                    FBig::<$mode, $base>::from_parts(significand.clone(), exponent).to_f32()
                });
                check_f32(
                    data, &fields, $mode_name, $round, false, &exact, observed, started,
                );
            }
        }};
    }

    match (radix, case.rounding % 4) {
        (2, 0) => probe!(HalfEven, 2, "nearest_even", Round::Nearest),
        (2, 1) => probe!(Up, 2, "up", Round::Up),
        (2, 2) => probe!(Down, 2, "down", Round::Down),
        (2, _) => probe!(Zero, 2, "toward_zero", Round::Zero),
        (10, 0) => {
            // Probe both the explicitly requested ties-to-even mode and Dashu's
            // public decimal alias, whose documented tie rule is half-away.
            probe!(HalfEven, 10, "nearest_even", Round::Nearest);
            let started = Instant::now();
            let mut dbig_fields = fields.clone();
            if let Some((_, source_type)) = dbig_fields
                .iter_mut()
                .find(|(key, _)| *key == "source_type")
            {
                *source_type = "DBig".to_owned();
            }
            if case.format & 1 == 0 {
                let observed = catch_backend(TARGET, "dbig_to_f64", data, &dbig_fields, || {
                    DBig::from_parts(significand, exponent).to_f64()
                });
                check_f64(
                    data,
                    &dbig_fields,
                    "nearest_away",
                    Round::Nearest,
                    true,
                    &exact,
                    observed,
                    started,
                );
            } else {
                let observed = catch_backend(TARGET, "dbig_to_f32", data, &dbig_fields, || {
                    DBig::from_parts(significand, exponent).to_f32()
                });
                check_f32(
                    data,
                    &dbig_fields,
                    "nearest_away",
                    Round::Nearest,
                    true,
                    &exact,
                    observed,
                    started,
                );
            }
        }
        (10, 1) => probe!(Up, 10, "up", Round::Up),
        (10, 2) => probe!(Down, 10, "down", Round::Down),
        (10, _) => probe!(Zero, 10, "toward_zero", Round::Zero),
        _ => unreachable!(),
    }
});

fn exact_rational(bytes: &[u8], negative: bool, radix: u32, exponent: i32) -> Rational {
    let mut significand = Integer::from_digits(bytes, Order::MsfBe);
    if negative {
        significand = -significand;
    }
    if exponent == 0 {
        return Rational::from(significand);
    }
    let scale = if radix == 2 {
        Integer::from(1u8) << exponent.unsigned_abs()
    } else {
        Integer::from(Integer::u_pow_u(10, exponent.unsigned_abs()))
    };
    if exponent > 0 {
        Rational::from(significand * scale)
    } else {
        Rational::from((significand, scale))
    }
}

fn oracle_f64(exact: &Rational, round: Round, ties_away: bool) -> f64 {
    let (mut value, previous) = Float::with_val_round(f64::MANTISSA_DIGITS, exact, round);
    value.subnormalize_ieee_round(previous, round);
    let nearest = value.to_f64_round(round);
    if !ties_away || !nearest.is_finite() {
        return nearest;
    }
    halfway_away_f64(exact, nearest)
}

fn oracle_f32(exact: &Rational, round: Round, ties_away: bool) -> f32 {
    let (mut value, previous) = Float::with_val_round(f32::MANTISSA_DIGITS, exact, round);
    value.subnormalize_ieee_round(previous, round);
    let nearest = value.to_f32_round(round);
    if !ties_away || !nearest.is_finite() {
        return nearest;
    }
    halfway_away_f32(exact, nearest)
}

fn halfway_away_f64(exact: &Rational, nearest: f64) -> f64 {
    let adjacent = if exact < &rational_f64(nearest) {
        next_down_f64(nearest)
    } else {
        next_up_f64(nearest)
    };
    if !adjacent.is_finite() {
        return nearest;
    }
    let nearest_distance = Rational::from((exact - rational_f64(nearest)).abs());
    let adjacent_distance = Rational::from((exact - rational_f64(adjacent)).abs());
    if nearest_distance == adjacent_distance {
        adjacent
    } else {
        nearest
    }
}

fn halfway_away_f32(exact: &Rational, nearest: f32) -> f32 {
    let adjacent = if exact < &rational_f32(nearest) {
        next_down_f32(nearest)
    } else {
        next_up_f32(nearest)
    };
    if !adjacent.is_finite() {
        return nearest;
    }
    let nearest_distance = Rational::from((exact - rational_f32(nearest)).abs());
    let adjacent_distance = Rational::from((exact - rational_f32(adjacent)).abs());
    if nearest_distance == adjacent_distance {
        adjacent
    } else {
        nearest
    }
}

fn check_f64(
    input: &[u8],
    fields: &[(&str, String)],
    mode: &str,
    round: Round,
    ties_away: bool,
    exact: &Rational,
    observed: Approximation<f64, Rounding>,
    started: Instant,
) {
    let expected = oracle_f64(exact, round, ties_away);
    let value = *observed.value_ref();
    let is_exact = value.is_finite() && rational_f64(value) == *exact;
    let tagged_exact = matches!(observed, Approximation::Exact(_));
    if value.to_bits() != expected.to_bits() || is_exact != tagged_exact {
        fail_conversion(
            "to_f64",
            input,
            fields,
            mode,
            format!("{expected} ({:#018x})", expected.to_bits()),
            format!("{observed:?}"),
            is_exact,
            classify_f64(expected),
            classify_f64(value),
            started,
        );
    }
}

fn check_f32(
    input: &[u8],
    fields: &[(&str, String)],
    mode: &str,
    round: Round,
    ties_away: bool,
    exact: &Rational,
    observed: Approximation<f32, Rounding>,
    started: Instant,
) {
    let expected = oracle_f32(exact, round, ties_away);
    let value = *observed.value_ref();
    let is_exact = value.is_finite() && rational_f32(value) == *exact;
    let tagged_exact = matches!(observed, Approximation::Exact(_));
    if value.to_bits() != expected.to_bits() || is_exact != tagged_exact {
        fail_conversion(
            "to_f32",
            input,
            fields,
            mode,
            format!("{expected} ({:#010x})", expected.to_bits()),
            format!("{observed:?}"),
            is_exact,
            classify_f32(expected),
            classify_f32(value),
            started,
        );
    }
}

fn fail_conversion(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    mode: &str,
    expected: String,
    observed: String,
    exact: bool,
    expected_class: &'static str,
    observed_class: &'static str,
    started: Instant,
) -> ! {
    let reason = if observed.starts_with("Exact") != exact {
        "provider approximation metadata is incorrect"
    } else {
        "raw backend conversion differs from exact-rational oracle"
    };
    fail(
        TARGET,
        operation,
        reason,
        input,
        &[
            fields,
            &[
                ("rounding", mode.to_owned()),
                ("expected", expected),
                ("raw_backend_result", observed.clone()),
                ("observed", observed),
                ("exactly_representable", exact.to_string()),
                ("expected_class", expected_class.to_owned()),
                ("observed_class", observed_class.to_owned()),
                ("duration_us", started.elapsed().as_micros().to_string()),
            ],
        ]
        .concat(),
    )
}

fn classify_f64(value: f64) -> &'static str {
    if value.is_infinite() {
        "infinity"
    } else if value == 0.0 {
        "signed_zero"
    } else if value.abs() < f64::MIN_POSITIVE {
        "subnormal"
    } else {
        "finite"
    }
}

fn classify_f32(value: f32) -> &'static str {
    if value.is_infinite() {
        "infinity"
    } else if value == 0.0 {
        "signed_zero"
    } else if value.abs() < f32::MIN_POSITIVE {
        "subnormal"
    } else {
        "finite"
    }
}

fn rational_f64(value: f64) -> Rational {
    Float::with_val(f64::MANTISSA_DIGITS, value)
        .to_rational()
        .unwrap()
}

fn rational_f32(value: f32) -> Rational {
    Float::with_val(f32::MANTISSA_DIGITS, value)
        .to_rational()
        .unwrap()
}

fn next_up_f64(value: f64) -> f64 {
    if value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    if value > 0.0 {
        f64::from_bits(value.to_bits() + 1)
    } else {
        f64::from_bits(value.to_bits() - 1)
    }
}

fn next_down_f64(value: f64) -> f64 {
    if value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    if value > 0.0 {
        f64::from_bits(value.to_bits() - 1)
    } else {
        f64::from_bits(value.to_bits() + 1)
    }
}

fn next_up_f32(value: f32) -> f32 {
    if value == f32::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f32::from_bits(1);
    }
    if value > 0.0 {
        f32::from_bits(value.to_bits() + 1)
    } else {
        f32::from_bits(value.to_bits() - 1)
    }
}

fn next_down_f32(value: f32) -> f32 {
    if value == f32::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f32::from_bits(1);
    }
    if value > 0.0 {
        f32::from_bits(value.to_bits() - 1)
    } else {
        f32::from_bits(value.to_bits() + 1)
    }
}
