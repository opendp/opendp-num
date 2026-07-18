#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
use opendp_num::{Add, DirectedBinary, Direction, Div, Error, Mul, Result, Rounded, Sub};
use opendp_num_fuzz::{
    BinaryCase, catch_backend, directed_direction, fail, special_f32, special_f64,
};

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(case) = BinaryCase::arbitrary(&mut unstructured) else {
        return;
    };
    let direction = directed_direction(case.direction);

    if case.format & 1 == 0 {
        let lhs = special_f64(case.lhs_selector, case.lhs_bits);
        let rhs = special_f64(case.rhs_selector, case.rhs_bits);
        let fields = fields_f64(lhs, rhs, direction);
        match case.operation % 4 {
            0 => run_f64::<Add>("add", lhs, rhs, direction, data, &fields),
            1 => run_f64::<Sub>("sub", lhs, rhs, direction, data, &fields),
            2 => run_f64::<Mul>("mul", lhs, rhs, direction, data, &fields),
            _ => run_f64::<Div>("div", lhs, rhs, direction, data, &fields),
        }
    } else {
        let lhs = special_f32(case.lhs_selector, case.lhs_bits);
        let rhs = special_f32(case.rhs_selector, case.rhs_bits);
        let fields = fields_f32(lhs, rhs, direction);
        match case.operation % 4 {
            0 => run_f32::<Add>("add", lhs, rhs, direction, data, &fields),
            1 => run_f32::<Sub>("sub", lhs, rhs, direction, data, &fields),
            2 => run_f32::<Mul>("mul", lhs, rhs, direction, data, &fields),
            _ => run_f32::<Div>("div", lhs, rhs, direction, data, &fields),
        }
    }
});

fn run_f64<Op>(
    operation: &str,
    lhs: f64,
    rhs: f64,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) where
    Dashu: DirectedBinary<Op, f64>,
    Mpfr: DirectedBinary<Op, f64>,
{
    let dashu = catch_backend("directed_binary", operation, input, fields, || {
        <Dashu as DirectedBinary<Op, f64>>::eval(lhs, rhs, direction)
    });
    let mpfr = catch_backend("directed_binary", operation, input, fields, || {
        <Mpfr as DirectedBinary<Op, f64>>::eval(lhs, rhs, direction)
    });
    compare_f64(operation, input, fields, dashu, mpfr);
    check_bounds_f64(
        operation,
        input,
        fields,
        <Dashu as DirectedBinary<Op, f64>>::eval(lhs, rhs, Direction::Down),
        <Dashu as DirectedBinary<Op, f64>>::eval(lhs, rhs, Direction::Up),
    );
}

fn run_f32<Op>(
    operation: &str,
    lhs: f32,
    rhs: f32,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) where
    Dashu: DirectedBinary<Op, f32>,
    Mpfr: DirectedBinary<Op, f32>,
{
    let dashu = catch_backend("directed_binary", operation, input, fields, || {
        <Dashu as DirectedBinary<Op, f32>>::eval(lhs, rhs, direction)
    });
    let mpfr = catch_backend("directed_binary", operation, input, fields, || {
        <Mpfr as DirectedBinary<Op, f32>>::eval(lhs, rhs, direction)
    });
    compare_f32(operation, input, fields, dashu, mpfr);
    check_bounds_f32(
        operation,
        input,
        fields,
        <Dashu as DirectedBinary<Op, f32>>::eval(lhs, rhs, Direction::Down),
        <Dashu as DirectedBinary<Op, f32>>::eval(lhs, rhs, Direction::Up),
    );
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
        (Err(dashu), Err(mpfr)) if dashu.kind == mpfr.kind => {}
        (dashu, mpfr) => fail(
            "directed_binary",
            operation,
            outcome_reason(&dashu, &mpfr),
            input,
            &[
                fields,
                &[
                    ("dashu", format_outcome_f64(&dashu)),
                    ("mpfr", format_outcome_f64(&mpfr)),
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
        (Err(dashu), Err(mpfr)) if dashu.kind == mpfr.kind => {}
        (dashu, mpfr) => fail(
            "directed_binary",
            operation,
            outcome_reason(&dashu, &mpfr),
            input,
            &[
                fields,
                &[
                    ("dashu", format_outcome_f32(&dashu)),
                    ("mpfr", format_outcome_f32(&mpfr)),
                ],
            ]
            .concat(),
        ),
    }
}

fn check_bounds_f64(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    down: Result<Rounded<f64>>,
    up: Result<Rounded<f64>>,
) {
    if let (Ok(down), Ok(up)) = (down, up) {
        if down.value > up.value {
            fail(
                "directed_binary",
                operation,
                "directed lower bound exceeds upper bound",
                input,
                &[
                    fields,
                    &[
                        (
                            "down",
                            format!("{} ({:#018x})", down.value, down.value.to_bits()),
                        ),
                        ("up", format!("{} ({:#018x})", up.value, up.value.to_bits())),
                    ],
                ]
                .concat(),
            );
        }
    }
}

fn check_bounds_f32(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    down: Result<Rounded<f32>>,
    up: Result<Rounded<f32>>,
) {
    if let (Ok(down), Ok(up)) = (down, up) {
        if down.value > up.value {
            fail(
                "directed_binary",
                operation,
                "directed lower bound exceeds upper bound",
                input,
                &[
                    fields,
                    &[
                        (
                            "down",
                            format!("{} ({:#010x})", down.value, down.value.to_bits()),
                        ),
                        ("up", format!("{} ({:#010x})", up.value, up.value.to_bits())),
                    ],
                ]
                .concat(),
            );
        }
    }
}

fn outcome_reason<T>(dashu: &Result<T>, mpfr: &Result<T>) -> &'static str {
    match (dashu, mpfr) {
        (Ok(_), Ok(_)) => "correctly rounded value differs from MPFR",
        (Ok(_), Err(_)) => "Dashu returned a value where MPFR reports an error",
        (Err(_), Ok(_)) => "Dashu returned an error for a valid MPFR result",
        (Err(_), Err(_)) => "Dashu and MPFR classify the error differently",
    }
}

fn format_outcome_f64(value: &Result<Rounded<f64>>) -> String {
    match value {
        Ok(value) => format!("{} ({:#018x})", value.value, value.value.to_bits()),
        Err(error) => format_error(error),
    }
}

fn format_outcome_f32(value: &Result<Rounded<f32>>) -> String {
    match value {
        Ok(value) => format!("{} ({:#010x})", value.value, value.value.to_bits()),
        Err(error) => format_error(error),
    }
}

fn format_error(error: &Error) -> String {
    format!("error {:?}: {}", error.kind, error.message)
}

fn fields_f64(lhs: f64, rhs: f64, direction: Direction) -> Vec<(&'static str, String)> {
    vec![
        ("format", "f64".to_owned()),
        ("lhs", lhs.to_string()),
        ("lhs_bits", format!("{:#018x}", lhs.to_bits())),
        ("rhs", rhs.to_string()),
        ("rhs_bits", format!("{:#018x}", rhs.to_bits())),
        ("direction", format!("{direction:?}")),
    ]
}

fn fields_f32(lhs: f32, rhs: f32, direction: Direction) -> Vec<(&'static str, String)> {
    vec![
        ("format", "f32".to_owned()),
        ("lhs", lhs.to_string()),
        ("lhs_bits", format!("{:#010x}", lhs.to_bits())),
        ("rhs", rhs.to_string()),
        ("rhs_bits", format!("{:#010x}", rhs.to_bits())),
        ("direction", format!("{direction:?}")),
    ]
}
