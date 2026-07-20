#![no_main]

use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
use opendp_num::{
    Add, DirectedBinary, DirectedPowI, DirectedUnary, Direction, Div, Exp, ExpM1, Ln, Ln1p, Log2,
    Mul, Result, Rounded, Sqrt, Sub,
};
use opendp_num_fuzz::{catch_backend, directed_direction, fail, special_exponent, special_f64};

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }
    let initial_bits = u64::from_le_bytes(data[1..9].try_into().unwrap());
    let mut dashu_value = special_f64(data[0], initial_bits);
    let mut mpfr_value = dashu_value;
    let mut trace = vec![format!("start({:#018x})", dashu_value.to_bits())];

    for (step, instruction) in data[9..].chunks(10).take(32).enumerate() {
        if instruction.len() < 10 {
            break;
        }
        let control = instruction[0];
        let selector = instruction[1];
        let operand_bits = u64::from_le_bytes(instruction[2..10].try_into().unwrap());
        let operand = special_f64(selector, operand_bits);
        let direction = directed_direction(control >> 4);
        let operation = control % 11;
        let exponent = special_exponent(selector, operand_bits as i32);
        let operation_name = operation_name(operation);
        let fields = [
            ("step", step.to_string()),
            ("operation", operation_name.to_owned()),
            ("direction", format!("{direction:?}")),
            (
                "dashu_input",
                format!("{} ({:#018x})", dashu_value, dashu_value.to_bits()),
            ),
            (
                "mpfr_input",
                format!("{} ({:#018x})", mpfr_value, mpfr_value.to_bits()),
            ),
            (
                "operand",
                format!("{} ({:#018x})", operand, operand.to_bits()),
            ),
            ("exponent", exponent.to_string()),
            ("trace", trace.join(" -> ")),
        ];

        let dashu = catch_backend("opendp_sequences", operation_name, data, &fields, || {
            dispatch_dashu(operation, dashu_value, operand, exponent, direction)
        });
        let mpfr = catch_backend("opendp_sequences", operation_name, data, &fields, || {
            dispatch_mpfr(operation, mpfr_value, operand, exponent, direction)
        });

        match (dashu, mpfr) {
            (Ok(dashu), Ok(mpfr)) => {
                if dashu.value.to_bits() != mpfr.value.to_bits() {
                    fail(
                        "opendp_sequences",
                        operation_name,
                        "composed correctly rounded result differs from MPFR",
                        data,
                        &[
                            fields.as_slice(),
                            &[
                                (
                                    "dashu_output",
                                    format!("{} ({:#018x})", dashu.value, dashu.value.to_bits()),
                                ),
                                (
                                    "mpfr_output",
                                    format!("{} ({:#018x})", mpfr.value, mpfr.value.to_bits()),
                                ),
                            ],
                        ]
                        .concat(),
                    );
                }
                dashu_value = dashu.value;
                mpfr_value = mpfr.value;
                trace.push(format!(
                    "{operation_name}[{direction:?}]({:#018x})",
                    dashu_value.to_bits()
                ));
            }
            (Err(dashu), Err(mpfr)) if dashu.kind == mpfr.kind => break,
            (dashu, mpfr) => fail(
                "opendp_sequences",
                operation_name,
                "composed operation success/error behavior differs from MPFR",
                data,
                &[
                    fields.as_slice(),
                    &[
                        ("dashu_output", format!("{dashu:?}")),
                        ("mpfr_output", format!("{mpfr:?}")),
                    ],
                ]
                .concat(),
            ),
        }
    }
});

fn dispatch_dashu(
    operation: u8,
    value: f64,
    operand: f64,
    exponent: i32,
    direction: Direction,
) -> Result<Rounded<f64>> {
    match operation {
        0 => <Dashu as DirectedBinary<Add, f64>>::eval(value, operand, direction),
        1 => <Dashu as DirectedBinary<Sub, f64>>::eval(value, operand, direction),
        2 => <Dashu as DirectedBinary<Mul, f64>>::eval(value, operand, direction),
        3 => <Dashu as DirectedBinary<Div, f64>>::eval(value, operand, direction),
        4 => <Dashu as DirectedUnary<Ln, f64>>::eval(value, direction),
        5 => <Dashu as DirectedUnary<Log2, f64>>::eval(value, direction),
        6 => <Dashu as DirectedUnary<Ln1p, f64>>::eval(value, direction),
        7 => <Dashu as DirectedUnary<Exp, f64>>::eval(value, direction),
        8 => <Dashu as DirectedUnary<ExpM1, f64>>::eval(value, direction),
        9 => <Dashu as DirectedUnary<Sqrt, f64>>::eval(value, direction),
        _ => <Dashu as DirectedPowI<f64>>::eval(value, &exponent, direction),
    }
}

fn dispatch_mpfr(
    operation: u8,
    value: f64,
    operand: f64,
    exponent: i32,
    direction: Direction,
) -> Result<Rounded<f64>> {
    match operation {
        0 => <Mpfr as DirectedBinary<Add, f64>>::eval(value, operand, direction),
        1 => <Mpfr as DirectedBinary<Sub, f64>>::eval(value, operand, direction),
        2 => <Mpfr as DirectedBinary<Mul, f64>>::eval(value, operand, direction),
        3 => <Mpfr as DirectedBinary<Div, f64>>::eval(value, operand, direction),
        4 => <Mpfr as DirectedUnary<Ln, f64>>::eval(value, direction),
        5 => <Mpfr as DirectedUnary<Log2, f64>>::eval(value, direction),
        6 => <Mpfr as DirectedUnary<Ln1p, f64>>::eval(value, direction),
        7 => <Mpfr as DirectedUnary<Exp, f64>>::eval(value, direction),
        8 => <Mpfr as DirectedUnary<ExpM1, f64>>::eval(value, direction),
        9 => <Mpfr as DirectedUnary<Sqrt, f64>>::eval(value, direction),
        _ => <Mpfr as DirectedPowI<f64>>::eval(value, &exponent, direction),
    }
}

fn operation_name(operation: u8) -> &'static str {
    match operation {
        0 => "add",
        1 => "sub",
        2 => "mul",
        3 => "div",
        4 => "ln",
        5 => "log2",
        6 => "ln1p",
        7 => "exp",
        8 => "expm1",
        9 => "sqrt",
        _ => "powi",
    }
}
