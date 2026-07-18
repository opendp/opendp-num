#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
use opendp_num::{
    DirectedPowI, DirectedUnary, Direction, Error, Exp, ExpM1, Ln, Ln1p, Log2, Result, Rounded,
    Sqrt,
};
use opendp_num_fuzz::{
    UnaryCase, catch_backend, directed_direction, fail, special_exponent, special_f32, special_f64,
};

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(case) = UnaryCase::arbitrary(&mut unstructured) else {
        return;
    };
    let direction = directed_direction(case.direction);
    let exponent = special_exponent(case.selector, case.exponent);

    if case.format & 1 == 0 {
        let value = special_f64(case.selector, case.bits);
        let fields = fields_f64(value, direction, exponent);
        match case.operation % 7 {
            0 => run_unary_f64::<Ln>("ln", value, direction, data, &fields),
            1 => run_unary_f64::<Log2>("log2", value, direction, data, &fields),
            2 => run_unary_f64::<Ln1p>("ln1p", value, direction, data, &fields),
            3 => run_unary_f64::<Exp>("exp", value, direction, data, &fields),
            4 => run_unary_f64::<ExpM1>("expm1", value, direction, data, &fields),
            5 => run_unary_f64::<Sqrt>("sqrt", value, direction, data, &fields),
            _ => run_powi_f64(value, exponent, direction, data, &fields),
        }
    } else {
        let value = special_f32(case.selector, case.bits);
        let fields = fields_f32(value, direction, exponent);
        match case.operation % 7 {
            0 => run_unary_f32::<Ln>("ln", value, direction, data, &fields),
            1 => run_unary_f32::<Log2>("log2", value, direction, data, &fields),
            2 => run_unary_f32::<Ln1p>("ln1p", value, direction, data, &fields),
            3 => run_unary_f32::<Exp>("exp", value, direction, data, &fields),
            4 => run_unary_f32::<ExpM1>("expm1", value, direction, data, &fields),
            5 => run_unary_f32::<Sqrt>("sqrt", value, direction, data, &fields),
            _ => run_powi_f32(value, exponent, direction, data, &fields),
        }
    }
});

fn run_unary_f64<Op>(
    operation: &str,
    value: f64,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) where
    Dashu: DirectedUnary<Op, f64>,
    Mpfr: DirectedUnary<Op, f64>,
{
    let dashu = catch_backend("directed_unary", operation, input, fields, || {
        <Dashu as DirectedUnary<Op, f64>>::eval(value, direction)
    });
    let mpfr = catch_backend("directed_unary", operation, input, fields, || {
        <Mpfr as DirectedUnary<Op, f64>>::eval(value, direction)
    });
    compare_f64(operation, input, fields, dashu, mpfr);

    let down = <Dashu as DirectedUnary<Op, f64>>::eval(value, Direction::Down);
    let up = <Dashu as DirectedUnary<Op, f64>>::eval(value, Direction::Up);
    check_bounds_f64(operation, input, fields, down, up);
}

fn run_unary_f32<Op>(
    operation: &str,
    value: f32,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) where
    Dashu: DirectedUnary<Op, f32>,
    Mpfr: DirectedUnary<Op, f32>,
{
    let dashu = catch_backend("directed_unary", operation, input, fields, || {
        <Dashu as DirectedUnary<Op, f32>>::eval(value, direction)
    });
    let mpfr = catch_backend("directed_unary", operation, input, fields, || {
        <Mpfr as DirectedUnary<Op, f32>>::eval(value, direction)
    });
    compare_f32(operation, input, fields, dashu, mpfr);

    let down = <Dashu as DirectedUnary<Op, f32>>::eval(value, Direction::Down);
    let up = <Dashu as DirectedUnary<Op, f32>>::eval(value, Direction::Up);
    check_bounds_f32(operation, input, fields, down, up);
}

fn run_powi_f64(
    value: f64,
    exponent: i32,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) {
    let dashu = catch_backend("directed_unary", "powi", input, fields, || {
        <Dashu as DirectedPowI<f64>>::eval(value, exponent, direction)
    });
    let mpfr = catch_backend("directed_unary", "powi", input, fields, || {
        <Mpfr as DirectedPowI<f64>>::eval(value, exponent, direction)
    });
    compare_f64("powi", input, fields, dashu, mpfr);
    check_bounds_f64(
        "powi",
        input,
        fields,
        <Dashu as DirectedPowI<f64>>::eval(value, exponent, Direction::Down),
        <Dashu as DirectedPowI<f64>>::eval(value, exponent, Direction::Up),
    );
}

fn run_powi_f32(
    value: f32,
    exponent: i32,
    direction: Direction,
    input: &[u8],
    fields: &[(&str, String)],
) {
    let dashu = catch_backend("directed_unary", "powi", input, fields, || {
        <Dashu as DirectedPowI<f32>>::eval(value, exponent, direction)
    });
    let mpfr = catch_backend("directed_unary", "powi", input, fields, || {
        <Mpfr as DirectedPowI<f32>>::eval(value, exponent, direction)
    });
    compare_f32("powi", input, fields, dashu, mpfr);
    check_bounds_f32(
        "powi",
        input,
        fields,
        <Dashu as DirectedPowI<f32>>::eval(value, exponent, Direction::Down),
        <Dashu as DirectedPowI<f32>>::eval(value, exponent, Direction::Up),
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
            "directed_unary",
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
            "directed_unary",
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
                "directed_unary",
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
                "directed_unary",
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

fn fields_f64(value: f64, direction: Direction, exponent: i32) -> Vec<(&'static str, String)> {
    vec![
        ("format", "f64".to_owned()),
        ("value", value.to_string()),
        ("value_bits", format!("{:#018x}", value.to_bits())),
        ("direction", format!("{direction:?}")),
        ("exponent", exponent.to_string()),
    ]
}

fn fields_f32(value: f32, direction: Direction, exponent: i32) -> Vec<(&'static str, String)> {
    vec![
        ("format", "f32".to_owned()),
        ("value", value.to_string()),
        ("value_bits", format!("{:#010x}", value.to_bits())),
        ("direction", format!("{direction:?}")),
        ("exponent", exponent.to_string()),
    ]
}
