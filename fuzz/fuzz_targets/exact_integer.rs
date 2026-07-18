#![no_main]

use std::{cmp::Ordering, str::FromStr};

use dashu::base::BitTest;
use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, malachite::Malachite, mpfr::Mpfr};
use opendp_num::{Add, ExactBinary, ExactUnary, Mul, Neg, Sub};
use opendp_num_fuzz::{catch_backend, fail, signed_decimal, split_evenly, unsigned_decimal};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let operation = data[0] % 12;
    let selectors = [data[1], data[2], data[3]];
    let signs = [data[4] & 1 != 0, data[5] & 1 != 0, data[6] & 1 != 0];
    let chunks = split_evenly(&data[7..], 3);

    if operation == 10 {
        let bytes = chunks[0];
        let value = catch_backend("exact_integer", "dashu_from_be_bytes", data, &[], || {
            dashu::integer::UBig::from_be_bytes(bytes)
        });
        let expected = independent_bit_length(bytes);
        let actual = value.bit_len();
        if actual != expected {
            fail(
                "exact_integer",
                "dashu_bit_len",
                "bit length differs from independent byte calculation",
                data,
                &[
                    ("expected", expected.to_string()),
                    ("actual", actual.to_string()),
                ],
            );
        }
        return;
    }

    let signed: Vec<String> = (0..3)
        .map(|index| signed_decimal(chunks[index], signs[index], selectors[index]))
        .collect();
    let unsigned: Vec<String> = (0..3)
        .map(|index| unsigned_decimal(chunks[index], selectors[index]))
        .collect();

    let dashu_signed: Vec<dashu::integer::IBig> = signed
        .iter()
        .map(|value| dashu::integer::IBig::from_str(value).unwrap())
        .collect();
    let malachite_signed: Vec<malachite::Integer> = signed
        .iter()
        .map(|value| malachite::Integer::from_str(value).unwrap())
        .collect();
    let rug_signed: Vec<rug::Integer> = signed
        .iter()
        .map(|value| rug::Integer::from_str(value).unwrap())
        .collect();

    let fields = [
        ("lhs", signed[0].clone()),
        ("rhs", signed[1].clone()),
        ("third", signed[2].clone()),
    ];

    match operation {
        0 => compare_signed(
            "add",
            data,
            &fields,
            <Dashu as ExactBinary<Add, _>>::eval(&dashu_signed[0], &dashu_signed[1]).to_string(),
            <Malachite as ExactBinary<Add, _>>::eval(&malachite_signed[0], &malachite_signed[1])
                .to_string(),
            <Mpfr as ExactBinary<Add, _>>::eval(&rug_signed[0], &rug_signed[1]).to_string(),
        ),
        1 => compare_signed(
            "sub",
            data,
            &fields,
            <Dashu as ExactBinary<Sub, _>>::eval(&dashu_signed[0], &dashu_signed[1]).to_string(),
            <Malachite as ExactBinary<Sub, _>>::eval(&malachite_signed[0], &malachite_signed[1])
                .to_string(),
            <Mpfr as ExactBinary<Sub, _>>::eval(&rug_signed[0], &rug_signed[1]).to_string(),
        ),
        2 => compare_signed(
            "mul",
            data,
            &fields,
            <Dashu as ExactBinary<Mul, _>>::eval(&dashu_signed[0], &dashu_signed[1]).to_string(),
            <Malachite as ExactBinary<Mul, _>>::eval(&malachite_signed[0], &malachite_signed[1])
                .to_string(),
            <Mpfr as ExactBinary<Mul, _>>::eval(&rug_signed[0], &rug_signed[1]).to_string(),
        ),
        3 => compare_signed(
            "neg",
            data,
            &fields,
            <Dashu as ExactUnary<Neg, _>>::eval(&dashu_signed[0]).to_string(),
            <Malachite as ExactUnary<Neg, _>>::eval(&malachite_signed[0]).to_string(),
            <Mpfr as ExactUnary<Neg, _>>::eval(&rug_signed[0]).to_string(),
        ),
        4 => {
            let d = dashu_signed[0].cmp(&dashu_signed[1]);
            let m = malachite_signed[0].cmp(&malachite_signed[1]);
            let r = rug_signed[0].cmp(&rug_signed[1]);
            if d != m || d != r {
                fail(
                    "exact_integer",
                    "compare",
                    "backend orderings disagree",
                    data,
                    &[
                        fields.as_slice(),
                        &[
                            ("dashu", ordering_name(d).to_owned()),
                            ("malachite", ordering_name(m).to_owned()),
                            ("mpfr", ordering_name(r).to_owned()),
                        ],
                    ]
                    .concat(),
                );
            }
        }
        5 | 6 => {
            if unsigned[1] == "0" {
                return;
            }
            let dl = dashu::integer::UBig::from_str(&unsigned[0]).unwrap();
            let dr = dashu::integer::UBig::from_str(&unsigned[1]).unwrap();
            let ml = malachite::Natural::from_str(&unsigned[0]).unwrap();
            let mr = malachite::Natural::from_str(&unsigned[1]).unwrap();
            let rl = rug::Integer::from_str(&unsigned[0]).unwrap();
            let rr = rug::Integer::from_str(&unsigned[1]).unwrap();
            let (name, d, m, r) = if operation == 5 {
                (
                    "natural_div",
                    (&dl / &dr).to_string(),
                    (&ml / &mr).to_string(),
                    rug::Integer::from(&rl / &rr).to_string(),
                )
            } else {
                (
                    "natural_rem",
                    (&dl % &dr).to_string(),
                    (&ml % &mr).to_string(),
                    rug::Integer::from(&rl % &rr).to_string(),
                )
            };
            compare_signed(
                name,
                data,
                &[("lhs", unsigned[0].clone()), ("rhs", unsigned[1].clone())],
                d,
                m,
                r,
            );
        }
        7 => {
            let d_sum = <Dashu as ExactBinary<Add, _>>::eval(&dashu_signed[0], &dashu_signed[1]);
            let d_back = <Dashu as ExactBinary<Sub, _>>::eval(&d_sum, &dashu_signed[1]);
            let m_sum = <Malachite as ExactBinary<Add, _>>::eval(
                &malachite_signed[0],
                &malachite_signed[1],
            );
            let m_back = <Malachite as ExactBinary<Sub, _>>::eval(&m_sum, &malachite_signed[1]);
            let r_sum = <Mpfr as ExactBinary<Add, _>>::eval(&rug_signed[0], &rug_signed[1]);
            let r_back = <Mpfr as ExactBinary<Sub, _>>::eval(&r_sum, &rug_signed[1]);
            compare_signed(
                "add_sub_inverse",
                data,
                &fields,
                d_back.to_string(),
                m_back.to_string(),
                r_back.to_string(),
            );
            if d_back != dashu_signed[0] || m_back != malachite_signed[0] || r_back != rug_signed[0]
            {
                fail(
                    "exact_integer",
                    "add_sub_inverse",
                    "(a + b) - b did not recover a",
                    data,
                    &fields,
                );
            }
        }
        8 => {
            let d_left = <Dashu as ExactBinary<Mul, _>>::eval(
                &dashu_signed[0],
                &<Dashu as ExactBinary<Add, _>>::eval(&dashu_signed[1], &dashu_signed[2]),
            );
            let d_right = <Dashu as ExactBinary<Add, _>>::eval(
                &<Dashu as ExactBinary<Mul, _>>::eval(&dashu_signed[0], &dashu_signed[1]),
                &<Dashu as ExactBinary<Mul, _>>::eval(&dashu_signed[0], &dashu_signed[2]),
            );
            if d_left != d_right {
                fail(
                    "exact_integer",
                    "distributivity",
                    "Dashu violated a * (b + c) = a*b + a*c",
                    data,
                    &fields,
                );
            }
        }
        9 => {
            let d_abs = if signed[0].starts_with('-') {
                -dashu_signed[0].clone()
            } else {
                dashu_signed[0].clone()
            };
            let m_abs = if malachite_signed[0] < 0 {
                -malachite_signed[0].clone()
            } else {
                malachite_signed[0].clone()
            };
            let r_abs = rug_signed[0].clone().abs();
            compare_signed(
                "abs",
                data,
                &fields,
                d_abs.to_string(),
                m_abs.to_string(),
                r_abs.to_string(),
            );
        }
        _ => {
            let dl = dashu::integer::UBig::from_str(&unsigned[0]).unwrap();
            let dr = dashu::integer::UBig::from_str(&unsigned[1]).unwrap();
            let ml = malachite::Natural::from_str(&unsigned[0]).unwrap();
            let mr = malachite::Natural::from_str(&unsigned[1]).unwrap();
            let rl = rug::Integer::from_str(&unsigned[0]).unwrap();
            let rr = rug::Integer::from_str(&unsigned[1]).unwrap();
            compare_signed(
                "gcd_euclid",
                data,
                &[("lhs", unsigned[0].clone()), ("rhs", unsigned[1].clone())],
                gcd_dashu(dl, dr).to_string(),
                gcd_malachite(ml, mr).to_string(),
                gcd_rug(rl, rr).to_string(),
            );
        }
    }
});

fn compare_signed(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    dashu: String,
    malachite: String,
    mpfr: String,
) {
    if dashu != malachite || dashu != mpfr {
        fail(
            "exact_integer",
            operation,
            "cross-backend exact result mismatch",
            input,
            &[
                fields,
                &[("dashu", dashu), ("malachite", malachite), ("mpfr", mpfr)],
            ]
            .concat(),
        );
    }
}

fn ordering_name(value: Ordering) -> &'static str {
    match value {
        Ordering::Less => "less",
        Ordering::Equal => "equal",
        Ordering::Greater => "greater",
    }
}

fn independent_bit_length(bytes: &[u8]) -> usize {
    let Some((first_index, first)) = bytes
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| *byte != 0)
    else {
        return 0;
    };
    (bytes.len() - first_index - 1) * 8 + (8 - first.leading_zeros() as usize)
}

fn gcd_dashu(mut lhs: dashu::integer::UBig, mut rhs: dashu::integer::UBig) -> dashu::integer::UBig {
    while rhs != dashu::integer::UBig::ZERO {
        let remainder = &lhs % &rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

fn gcd_malachite(mut lhs: malachite::Natural, mut rhs: malachite::Natural) -> malachite::Natural {
    while rhs != malachite::Natural::from(0u8) {
        let remainder = &lhs % &rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

fn gcd_rug(mut lhs: rug::Integer, mut rhs: rug::Integer) -> rug::Integer {
    while rhs != 0 {
        let remainder = rug::Integer::from(&lhs % &rhs);
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}
