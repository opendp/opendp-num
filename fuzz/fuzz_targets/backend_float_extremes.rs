#![no_main]

//! Raw dashu-float boundary fuzzing. This target deliberately bypasses the
//! opendp-num adapter and checks algebraic/range invariants that do not require
//! trusting another floating-point implementation.

use std::panic::{AssertUnwindSafe, catch_unwind};

use arbitrary::{Arbitrary, Unstructured};
use dashu::{
    base::{BitTest, Sign},
    float::{
        FBig,
        round::mode::{Down, Up},
    },
    integer::IBig,
};
use libfuzzer_sys::fuzz_target;
use opendp_num_fuzz::{fail, special_f64};

const TARGET: &str = "backend_float_extremes";

#[derive(Arbitrary, Debug)]
struct RawExtremeCase {
    operation: u8,
    precision_selector: u8,
    input_selector: u8,
    bits: u64,
    base_exponent_selector: u8,
    exponent_selector: u8,
    offset: i16,
    negative: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(case) = RawExtremeCase::arbitrary(&mut unstructured) else {
        return;
    };
    match case.operation % 3 {
        0 => probe_exp(&case, data, false),
        1 => probe_exp(&case, data, true),
        _ => probe_power_of_two(&case, data),
    }
});

fn precision(selector: u8) -> usize {
    const PRECISIONS: [usize; 8] = [1, 2, 3, 8, 24, 53, 128, 256];
    PRECISIONS[selector as usize % PRECISIONS.len()]
}

fn fields(operation: &str, case: &RawExtremeCase) -> Vec<(&'static str, String)> {
    vec![
        ("contract", "backend_conformance".to_owned()),
        ("provider", "dashu".to_owned()),
        ("owner", "backend".to_owned()),
        ("construction", "raw_FBig".to_owned()),
        ("source_type", "FBig<_,2>".to_owned()),
        ("operation", operation.to_owned()),
        (
            "source_precision",
            precision(case.precision_selector).to_string(),
        ),
        ("oracle", "exact_boundary_invariants".to_owned()),
        ("masked_by_adapter", "true".to_owned()),
    ]
}

fn fail_case(
    operation: &str,
    reason: &str,
    data: &[u8],
    case: &RawExtremeCase,
    details: &[(&str, String)],
) -> ! {
    fail(
        TARGET,
        operation,
        reason,
        data,
        &[fields(operation, case), details.to_vec()].concat(),
    )
}

fn probe_exp(case: &RawExtremeCase, data: &[u8], minus_one: bool) {
    let operation = if minus_one { "exp_m1" } else { "exp" };
    let value = special_f64(case.input_selector, case.bits);
    if !value.is_finite() {
        return;
    }
    // The isize range-reduction boundary has dedicated subprocess coverage:
    // DASHU-023 checks its directed saturation and DASHU-026 checks the
    // debug/fuzz-profile exp_m1 panic. libFuzzer aborts instead of unwinding,
    // so classify this known-danger region before invoking Dashu.
    let saturation_threshold = -(isize::MAX as f64) * std::f64::consts::LN_2;
    if value <= saturation_threshold {
        return;
    }
    // Debug/fuzz builds allocate roughly in proportion to a large-magnitude
    // exp_m1 input while aligning the subtraction of one (DASHU-027). Keep
    // mutation workers below a bounded 1e6 input; the dedicated allocation
    // reproducer measures the excluded resource-growth class in isolation.
    if minus_one && value.abs() > 1_000_000.0 {
        return;
    }
    let precision = precision(case.precision_selector);
    let up_input = FBig::<Up>::try_from(value)
        .unwrap()
        .with_precision(precision)
        .value();
    let down_input = FBig::<Down>::try_from(value)
        .unwrap()
        .with_precision(precision)
        .value();
    let up = catch_unwind(AssertUnwindSafe(|| {
        if minus_one {
            up_input.exp_m1()
        } else {
            up_input.exp()
        }
    }));
    let down = catch_unwind(AssertUnwindSafe(|| {
        if minus_one {
            down_input.exp_m1()
        } else {
            down_input.exp()
        }
    }));
    let (Ok(up), Ok(down)) = (up, down) else {
        fail_case(
            operation,
            "backend panic outside an allowed exact-special-case finding",
            data,
            case,
            &[("input", value.to_string())],
        );
    };

    // Repr ordering rejects infinity operands. Infinity is a valid public
    // FBig result here, so only use the finite ordering operation on finite
    // endpoints.
    if !up.repr().is_infinite() && !down.repr().is_infinite() && up.repr() < down.repr() {
        fail_case(
            operation,
            "upward result is below downward result",
            data,
            case,
            &[("input", value.to_string())],
        );
    }

    // DASHU-023 covers the known range-reduction saturation once x/ln(2)
    // no longer fits isize. Keep exercising it without terminating every
    // campaign; any premature instance still fails.
    if minus_one {
        if value < 0.0 && up <= FBig::<Up>::NEG_ONE {
            fail_case(
                operation,
                "upward exp_m1 is not above -1 for finite input",
                data,
                case,
                &[("input", value.to_string())],
            );
        }
    } else {
        if !up.repr().is_infinite() && up.repr().significand().is_zero() {
            fail_case(
                operation,
                "upward exp is zero for finite input",
                data,
                case,
                &[("input", value.to_string())],
            );
        }
        if up.repr().sign() == Sign::Negative || down.repr().sign() == Sign::Negative {
            fail_case(
                operation,
                "exp produced a negative result",
                data,
                case,
                &[("input", value.to_string())],
            );
        }
    }
}

fn selected_base_exponent(selector: u8, offset: i16) -> isize {
    const BASES: [isize; 9] = [-1074, -149, -2, -1, 0, 1, 2, 127, 1023];
    BASES[selector as usize % BASES.len()].saturating_add(isize::from(offset.clamp(-4, 4)))
}

fn selected_power(selector: u8, bits: u64, offset: i16) -> IBig {
    let delta = i64::from(offset.clamp(-4, 4));
    match selector % 8 {
        0 => IBig::from(delta),
        1 => IBig::from(i32::MAX as i64 + delta),
        2 => IBig::from(10_000_000_000_000_000i64.saturating_add(delta)),
        3 => IBig::from(isize::MAX).add(delta),
        4 => IBig::from(isize::MIN).add(delta),
        5 => IBig::from(bits as i64),
        6 => IBig::from(-(bits as i64)),
        _ => (IBig::ONE << 200) + IBig::from(delta),
    }
}

fn probe_power_of_two(case: &RawExtremeCase, data: &[u8]) {
    let precision = precision(case.precision_selector);
    let base_exponent = selected_base_exponent(case.base_exponent_selector, case.offset);
    let exponent = selected_power(case.exponent_selector, case.bits, case.offset);
    let exponent_i128 = i128::try_from(&exponent).ok();
    let exact_exponent = exponent_i128.and_then(|value| (base_exponent as i128).checked_mul(value));
    let exact_in_range = exact_exponent.and_then(|value| isize::try_from(value).ok());
    let odd = exponent.clone().into_parts().1.bit(0);
    let negative = case.negative && odd;
    let details = [
        ("base_exponent", base_exponent.to_string()),
        ("exponent", exponent.to_string()),
        ("negative_result", negative.to_string()),
    ];

    // libFuzzer builds use abort-on-panic, so catch_unwind cannot isolate a
    // known crashing case. These range classes have dedicated normal-build
    // subprocess reproducers (DASHU-024/025); exclude them *before* powi and
    // keep this target alive to search the remaining representable domain.
    let Some(expected_exponent) = exact_in_range else {
        return;
    };
    // A negative exponent is implemented as positive powi followed by a
    // reciprocal. The final exponent -isize::MAX therefore also drives the
    // intermediate power to the +isize::MAX DASHU-024 boundary.
    let reciprocal_boundary =
        exponent.sign() == Sign::Negative && expected_exponent == isize::MAX.saturating_neg();
    if expected_exponent == isize::MAX || expected_exponent == isize::MIN || reciprocal_boundary {
        return;
    }

    let base_significand = if case.negative { -IBig::ONE } else { IBig::ONE };
    let up_base = FBig::<Up>::from_parts(base_significand.clone(), base_exponent)
        .with_precision(precision)
        .value();
    let down_base = FBig::<Down>::from_parts(base_significand, base_exponent)
        .with_precision(precision)
        .value();
    let up = up_base.powi(exponent.clone());
    let down = down_base.powi(exponent);
    let expected_significand = if negative { -IBig::ONE } else { IBig::ONE };
    for (mode, result) in [("up", up.repr()), ("down", down.repr())] {
        if result.is_infinite()
            || result.significand() != &expected_significand
            || result.exponent() != expected_exponent
        {
            fail_case(
                "powi",
                "exact power-of-two result differs from structural oracle",
                data,
                case,
                &[
                    details.as_slice(),
                    &[
                        ("rounding", mode.to_owned()),
                        ("expected_exponent", expected_exponent.to_string()),
                    ],
                ]
                .concat(),
            );
        }
    }
}

trait AddSmall {
    fn add(self, value: i64) -> Self;
}

impl AddSmall for IBig {
    fn add(self, value: i64) -> Self {
        self + IBig::from(value)
    }
}
