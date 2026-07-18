#![no_main]

use std::str::FromStr;

use libfuzzer_sys::fuzz_target;
use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
use opendp_num::{
    Add, CheckedBinary, Div, ExactBinary, ExactUnary, FromParts, IntoParts, Mul, Neg, Sub,
};
use opendp_num_fuzz::{fail, signed_decimal, split_evenly, unsigned_decimal};

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }
    let operation = data[0] % 11;
    let selectors = [data[1], data[2], data[3], data[4]];
    let signs = [data[5] & 1 != 0, data[6] & 1 != 0];
    let chunks = split_evenly(&data[7..], 4);

    let numerator_a = signed_decimal(chunks[0], signs[0], selectors[0]);
    let numerator_b = signed_decimal(chunks[1], signs[1], selectors[1]);
    let mut denominator_a = unsigned_decimal(chunks[2], selectors[2]);
    let mut denominator_b = unsigned_decimal(chunks[3], selectors[3]);
    if denominator_a == "0" {
        denominator_a = "1".to_owned();
    }
    if denominator_b == "0" {
        denominator_b = "1".to_owned();
    }

    let dashu_a = <Dashu as FromParts<
        dashu::rational::RBig,
        dashu::integer::IBig,
        dashu::integer::UBig,
    >>::from_parts(
        dashu::integer::IBig::from_str(&numerator_a).unwrap(),
        dashu::integer::UBig::from_str(&denominator_a).unwrap(),
    )
    .unwrap();
    let dashu_b = <Dashu as FromParts<
        dashu::rational::RBig,
        dashu::integer::IBig,
        dashu::integer::UBig,
    >>::from_parts(
        dashu::integer::IBig::from_str(&numerator_b).unwrap(),
        dashu::integer::UBig::from_str(&denominator_b).unwrap(),
    )
    .unwrap();

    let rug_a = <Mpfr as FromParts<rug::Rational, rug::Integer, rug::Integer>>::from_parts(
        rug::Integer::from_str(&numerator_a).unwrap(),
        rug::Integer::from_str(&denominator_a).unwrap(),
    )
    .unwrap();
    let rug_b = <Mpfr as FromParts<rug::Rational, rug::Integer, rug::Integer>>::from_parts(
        rug::Integer::from_str(&numerator_b).unwrap(),
        rug::Integer::from_str(&denominator_b).unwrap(),
    )
    .unwrap();

    let fields = [
        ("numerator_a", numerator_a),
        ("denominator_a", denominator_a),
        ("numerator_b", numerator_b),
        ("denominator_b", denominator_b),
    ];

    match operation {
        0 => compare(
            "add",
            data,
            &fields,
            &<Dashu as ExactBinary<Add, _>>::eval(&dashu_a, &dashu_b),
            &<Mpfr as ExactBinary<Add, _>>::eval(&rug_a, &rug_b),
        ),
        1 => compare(
            "sub",
            data,
            &fields,
            &<Dashu as ExactBinary<Sub, _>>::eval(&dashu_a, &dashu_b),
            &<Mpfr as ExactBinary<Sub, _>>::eval(&rug_a, &rug_b),
        ),
        2 => compare(
            "mul",
            data,
            &fields,
            &<Dashu as ExactBinary<Mul, _>>::eval(&dashu_a, &dashu_b),
            &<Mpfr as ExactBinary<Mul, _>>::eval(&rug_a, &rug_b),
        ),
        3 => {
            let dashu = <Dashu as CheckedBinary<Div, _>>::eval(&dashu_a, &dashu_b);
            let rug = <Mpfr as CheckedBinary<Div, _>>::eval(&rug_a, &rug_b);
            match (dashu, rug) {
                (Ok(dashu), Ok(rug)) => compare("div", data, &fields, &dashu, &rug),
                (Err(dashu), Err(rug)) if dashu.kind == rug.kind => {}
                (dashu, rug) => fail(
                    "exact_rational",
                    "div",
                    "division success/error behavior differs",
                    data,
                    &[
                        fields.as_slice(),
                        &[
                            ("dashu", format!("{dashu:?}")),
                            ("mpfr", format!("{rug:?}")),
                        ],
                    ]
                    .concat(),
                ),
            }
        }
        4 => compare(
            "neg",
            data,
            &fields,
            &<Dashu as ExactUnary<Neg, _>>::eval(&dashu_a),
            &<Mpfr as ExactUnary<Neg, _>>::eval(&rug_a),
        ),
        5 => {
            let dashu_key = dashu_key(dashu_a.clone());
            let rug_key = rug_key(rug_a.clone());
            if dashu_key != rug_key {
                fail(
                    "exact_rational",
                    "canonical_parts",
                    "canonical numerator/denominator differ",
                    data,
                    &[
                        fields.as_slice(),
                        &[("dashu", dashu_key), ("mpfr", rug_key)],
                    ]
                    .concat(),
                );
            }
        }
        6 => {
            let dashu = dashu_a.clone().floor().to_string();
            let rug = rug_a.clone().floor().to_string();
            if dashu != rug {
                fail(
                    "exact_rational",
                    "floor",
                    "floor differs from MPFR rational",
                    data,
                    &[fields.as_slice(), &[("dashu", dashu), ("mpfr", rug)]].concat(),
                );
            }
        }
        7 => {
            let dashu = dashu_a.clone().pow(2);
            let rug = rug::Rational::from(&rug_a * &rug_a);
            compare("pow2", data, &fields, &dashu, &rug);
        }
        8 => {
            let dashu_order = dashu_a.cmp(&dashu_b);
            let rug_order = rug_a.cmp(&rug_b);
            if dashu_order != rug_order {
                fail(
                    "exact_rational",
                    "compare",
                    "rational ordering differs",
                    data,
                    &[
                        fields.as_slice(),
                        &[
                            ("dashu", format!("{dashu_order:?}")),
                            ("mpfr", format!("{rug_order:?}")),
                        ],
                    ]
                    .concat(),
                );
            }
        }
        9 => {
            let dashu = dashu_a.round().to_string();
            let expected = round_rational_away_from_zero(&fields[0].1, &fields[1].1);
            if dashu != expected {
                fail(
                    "exact_rational",
                    "round_away_from_zero",
                    "nearest rational-to-integer rounding differs; ties must round away from zero",
                    data,
                    &[
                        fields.as_slice(),
                        &[("dashu", dashu), ("expected", expected)],
                    ]
                    .concat(),
                );
            }
        }
        _ => {
            let dashu_square = <Dashu as ExactBinary<Mul, _>>::eval(&dashu_a, &dashu_a);
            let rug_square = <Mpfr as ExactBinary<Mul, _>>::eval(&rug_a, &rug_a);
            compare("square_via_mul", data, &fields, &dashu_square, &rug_square);
        }
    }
});

fn compare(
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    dashu: &dashu::rational::RBig,
    rug: &rug::Rational,
) {
    let dashu = dashu_key(dashu.clone());
    let rug = rug_key(rug.clone());
    if dashu != rug {
        fail(
            "exact_rational",
            operation,
            "exact rational result differs from MPFR",
            input,
            &[fields, &[("dashu", dashu), ("mpfr", rug)]].concat(),
        );
    }
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

fn round_rational_away_from_zero(numerator: &str, denominator: &str) -> String {
    let numerator = rug::Integer::from_str(numerator).unwrap();
    let denominator = rug::Integer::from_str(denominator).unwrap();
    let mut quotient = rug::Integer::from(&numerator / &denominator);
    let remainder = rug::Integer::from(&numerator % &denominator);
    let mut twice_remainder = if remainder < 0 { -remainder } else { remainder };
    twice_remainder <<= 1;
    if twice_remainder >= denominator {
        if numerator > 0 {
            quotient += 1;
        } else if numerator < 0 {
            quotient -= 1;
        }
    }
    quotient.to_string()
}
