#![no_main]

use std::str::FromStr;

use dashu::float::{
    FBig,
    round::{
        Round,
        mode::{Down, Up, Zero},
    },
};
use libfuzzer_sys::fuzz_target;
use opendp_num_fuzz::{catch_backend, fail, special_f64};
use rug::{Integer, Rational};

fuzz_target!(|data: &[u8]| {
    if data.len() < 29 {
        return;
    }

    let operation = data[0] % 5;
    let selector_a = data[1];
    let selector_b = data[2];
    let precision = usize::from(u16::from_le_bytes(data[3..5].try_into().unwrap()) % 256) + 1;
    let bits_a = u64::from_le_bytes(data[5..13].try_into().unwrap());
    let bits_b = u64::from_le_bytes(data[13..21].try_into().unwrap());
    let integer = u64::from_le_bytes(data[21..29].try_into().unwrap());
    let value_a = special_f64(selector_a, bits_a);
    let value_b = special_f64(selector_b, bits_b);

    match operation {
        0 => fuzz_precision_rounding(value_a, precision, data),
        1 => fuzz_floor_fraction(value_a, precision, data),
        2 => fuzz_reciprocal_probability(value_a, data),
        3 => fuzz_parameter_comparison(value_a, value_b, data),
        _ => fuzz_scale_round_pipeline(value_a, value_b, integer, precision, data),
    }
});

fn fuzz_precision_rounding(value: f64, precision: usize, input: &[u8]) {
    if !value.is_finite() {
        return;
    }
    let fields = [
        ("value", value.to_string()),
        ("value_bits", format!("{:#018x}", value.to_bits())),
        ("precision", precision.to_string()),
    ];

    let down = catch_backend(
        "alp_primitives",
        "with_precision_down",
        input,
        &fields,
        || {
            FBig::<Down>::try_from(value)
                .expect("finite primitive conversion is exact")
                .with_precision(precision)
                .value()
        },
    );
    let up = catch_backend(
        "alp_primitives",
        "with_precision_up",
        input,
        &fields,
        || {
            FBig::<Up>::try_from(value)
                .expect("finite primitive conversion is exact")
                .with_precision(precision)
                .value()
        },
    );
    let zero = catch_backend(
        "alp_primitives",
        "with_precision_zero",
        input,
        &fields,
        || {
            FBig::<Zero>::try_from(value)
                .expect("finite primitive conversion is exact")
                .with_precision(precision)
                .value()
        },
    );

    for (name, actual) in [
        ("down", down.precision()),
        ("up", up.precision()),
        ("zero", zero.precision()),
    ] {
        if actual != precision {
            fail(
                "alp_primitives",
                "with_precision",
                "result context did not retain the requested precision",
                input,
                &[
                    fields.as_slice(),
                    &[
                        ("mode", name.to_owned()),
                        ("actual_precision", actual.to_string()),
                    ],
                ]
                .concat(),
            );
        }
    }

    let exact = Rational::from_f64(value).expect("finite f64 is rational");
    let down_r = fbig_to_rational(&down);
    let up_r = fbig_to_rational(&up);
    let zero_r = fbig_to_rational(&zero);

    if down_r > exact {
        fail_rounding_bound(
            "with_precision_down",
            "down-rounded value exceeds exact input",
            input,
            &fields,
            &down_r,
            &exact,
            &down,
        );
    }
    if up_r < exact {
        fail_rounding_bound(
            "with_precision_up",
            "up-rounded value is below exact input",
            input,
            &fields,
            &up_r,
            &exact,
            &up,
        );
    }
    let zero_wrong = if value.is_sign_negative() {
        zero_r < exact
    } else {
        zero_r > exact
    };
    if zero_wrong {
        fail_rounding_bound(
            "with_precision_zero",
            "zero-rounded value is not toward zero",
            input,
            &fields,
            &zero_r,
            &exact,
            &zero,
        );
    }
    if down_r > zero_r || zero_r > up_r {
        fail(
            "alp_primitives",
            "with_precision",
            "round-toward-zero result is outside directed bounds",
            input,
            &[
                fields.as_slice(),
                &[
                    ("down", down_r.to_string()),
                    ("zero", zero_r.to_string()),
                    ("up", up_r.to_string()),
                ],
            ]
            .concat(),
        );
    }
}

fn fuzz_floor_fraction(value: f64, precision: usize, input: &[u8]) {
    let Some(value) = nonnegative_finite(value) else {
        return;
    };
    let fields = [
        ("value", value.to_string()),
        ("value_bits", format!("{:#018x}", value.to_bits())),
        ("precision", precision.to_string()),
    ];
    let rounded = catch_backend("alp_primitives", "floor_fraction", input, &fields, || {
        FBig::<Down>::try_from(value)
            .expect("finite primitive conversion is exact")
            .with_precision(precision)
            .value()
    });
    check_floor_fraction("floor_fraction", rounded, input, &fields);
}

fn fuzz_reciprocal_probability(alpha: f64, input: &[u8]) {
    let Some(alpha) = positive_finite(alpha) else {
        return;
    };
    let fields = [
        ("alpha", alpha.to_string()),
        ("alpha_bits", format!("{:#018x}", alpha.to_bits())),
    ];
    let probability = catch_backend(
        "alp_primitives",
        "reciprocal_probability",
        input,
        &fields,
        || {
            let denominator =
                FBig::<Down>::try_from(alpha).expect("finite primitive conversion is exact") + 2u8;
            FBig::<Up>::ONE / denominator.with_rounding::<Up>()
        },
    );

    let probability_r = fbig_to_rational(&probability);
    let alpha_r = Rational::from_f64(alpha).expect("finite f64 is rational");
    let exact =
        Rational::from((Integer::from(1), Integer::from(1))) / (alpha_r + Rational::from(2));
    let zero = Rational::from(0);
    let half = Rational::from((Integer::from(1), Integer::from(2)));

    if probability_r < exact {
        fail(
            "alp_primitives",
            "reciprocal_probability",
            "privacy probability rounded below the exact reciprocal",
            input,
            &[
                fields.as_slice(),
                &[
                    ("actual", probability_r.to_string()),
                    ("exact", exact.to_string()),
                    ("dashu_repr", fbig_repr(&probability)),
                ],
            ]
            .concat(),
        );
    }
    if probability_r <= zero || probability_r > half {
        fail(
            "alp_primitives",
            "reciprocal_probability",
            "probability is outside (0, 1/2]",
            input,
            &[
                fields.as_slice(),
                &[
                    ("actual", probability_r.to_string()),
                    ("dashu_repr", fbig_repr(&probability)),
                ],
            ]
            .concat(),
        );
    }
}

fn fuzz_parameter_comparison(scale: f64, alpha: f64, input: &[u8]) {
    let (Some(scale), Some(alpha)) = (positive_finite(scale), positive_finite(alpha)) else {
        return;
    };
    let fields = [
        ("scale", scale.to_string()),
        ("scale_bits", format!("{:#018x}", scale.to_bits())),
        ("alpha", alpha.to_string()),
        ("alpha_bits", format!("{:#018x}", alpha.to_bits())),
    ];
    let actual = catch_backend(
        "alp_primitives",
        "parameter_comparison",
        input,
        &fields,
        || {
            let scale =
                FBig::<Zero>::try_from(scale).expect("finite primitive conversion is exact");
            let alpha =
                FBig::<Zero>::try_from(alpha).expect("finite primitive conversion is exact");
            scale * (1i64 << 52) < alpha
        },
    );

    let scale_r = Rational::from_f64(scale).expect("finite f64 is rational");
    let alpha_r = Rational::from_f64(alpha).expect("finite f64 is rational");
    let expected = scale_r * Rational::from(Integer::from(1) << 52u32) < alpha_r;
    if actual != expected {
        fail(
            "alp_primitives",
            "parameter_comparison",
            "FBig comparison differs from exact rational comparison",
            input,
            &[
                fields.as_slice(),
                &[
                    ("actual", actual.to_string()),
                    ("expected", expected.to_string()),
                ],
            ]
            .concat(),
        );
    }
}

fn fuzz_scale_round_pipeline(scale: f64, alpha: f64, integer: u64, precision: usize, input: &[u8]) {
    let (Some(scale), Some(alpha)) = (positive_finite(scale), positive_finite(alpha)) else {
        return;
    };
    let fields = [
        ("scale", scale.to_string()),
        ("scale_bits", format!("{:#018x}", scale.to_bits())),
        ("alpha", alpha.to_string()),
        ("alpha_bits", format!("{:#018x}", alpha.to_bits())),
        ("integer", integer.to_string()),
        ("precision", precision.to_string()),
    ];

    let (quotient, truncated, product) = catch_backend(
        "alp_primitives",
        "scale_round_pipeline",
        input,
        &fields,
        || {
            let quotient = FBig::<Down>::try_from(scale)
                .expect("finite primitive conversion is exact")
                / FBig::<Down>::try_from(alpha).expect("finite primitive conversion is exact");
            let truncated = quotient.clone().with_precision(precision).value();
            let integer = FBig::<Down>::from(integer).with_precision(64).value();
            let product = integer * truncated.clone();
            (quotient, truncated, product)
        },
    );

    let exact_quotient = Rational::from_f64(scale).unwrap() / Rational::from_f64(alpha).unwrap();
    let quotient_r = fbig_to_rational(&quotient);
    let truncated_r = fbig_to_rational(&truncated);
    if quotient_r > exact_quotient {
        fail(
            "alp_primitives",
            "scale_round_pipeline",
            "down-rounded scale/alpha exceeds exact quotient",
            input,
            &[
                fields.as_slice(),
                &[
                    ("quotient", quotient_r.to_string()),
                    ("exact_quotient", exact_quotient.to_string()),
                ],
            ]
            .concat(),
        );
    }
    if truncated_r > quotient_r {
        fail(
            "alp_primitives",
            "scale_round_pipeline",
            "precision truncation in Down mode increased the quotient",
            input,
            &[
                fields.as_slice(),
                &[
                    ("before", quotient_r.to_string()),
                    ("after", truncated_r.to_string()),
                ],
            ]
            .concat(),
        );
    }
    check_floor_fraction("scale_round_pipeline", product, input, &fields);
}

fn check_floor_fraction<R: Round>(
    operation: &str,
    value: FBig<R>,
    input: &[u8],
    fields: &[(&str, String)],
) {
    let floor = catch_backend("alp_primitives", operation, input, fields, || value.floor());
    let fraction = catch_backend("alp_primitives", operation, input, fields, || value.fract());
    let value_r = fbig_to_rational(&value);
    let floor_r = fbig_to_rational(&floor);
    let fraction_r = fbig_to_rational(&fraction);
    let expected_floor = Rational::from(value_r.clone().floor());
    let reconstructed = Rational::from(&floor_r + &fraction_r);

    if floor_r != expected_floor {
        fail(
            "alp_primitives",
            operation,
            "FBig floor differs from exact rational floor",
            input,
            &[
                fields,
                &[
                    ("value", value_r.to_string()),
                    ("floor", floor_r.to_string()),
                    ("expected_floor", expected_floor.to_string()),
                    ("dashu_repr", fbig_repr(&value)),
                ],
            ]
            .concat(),
        );
    }
    if reconstructed != value_r {
        fail(
            "alp_primitives",
            operation,
            "floor + fraction does not reconstruct the FBig value exactly",
            input,
            &[
                fields,
                &[
                    ("value", value_r.to_string()),
                    ("floor", floor_r.to_string()),
                    ("fraction", fraction_r.to_string()),
                    ("reconstructed", reconstructed.to_string()),
                    ("dashu_repr", fbig_repr(&value)),
                ],
            ]
            .concat(),
        );
    }
    if fraction_r < 0 || fraction_r >= 1 {
        fail(
            "alp_primitives",
            operation,
            "fractional part of a nonnegative FBig is outside [0, 1)",
            input,
            &[
                fields,
                &[
                    ("fraction", fraction_r.to_string()),
                    ("dashu_repr", fbig_repr(&value)),
                ],
            ]
            .concat(),
        );
    }
}

fn fbig_to_rational<R: Round>(value: &FBig<R>) -> Rational {
    let (significand, exponent) = value.clone().into_repr().into_parts();
    let mut numerator = Integer::from_str(&significand.to_string()).unwrap();
    let mut denominator = Integer::from(1);
    if exponent >= 0 {
        numerator <<= u32::try_from(exponent).expect("fuzzed FBig exponent fits u32");
    } else {
        denominator <<= u32::try_from(-exponent).expect("fuzzed FBig exponent fits u32");
    }
    Rational::from((numerator, denominator))
}

fn fbig_repr<R: Round>(value: &FBig<R>) -> String {
    format!(
        "significand={} exponent={} precision={}",
        value.repr().significand(),
        value.repr().exponent(),
        value.precision()
    )
}

fn fail_rounding_bound<R: Round>(
    operation: &str,
    reason: &str,
    input: &[u8],
    fields: &[(&str, String)],
    rounded: &Rational,
    exact: &Rational,
    dashu: &FBig<R>,
) -> ! {
    fail(
        "alp_primitives",
        operation,
        reason,
        input,
        &[
            fields,
            &[
                ("rounded", rounded.to_string()),
                ("exact", exact.to_string()),
                ("dashu_repr", fbig_repr(dashu)),
            ],
        ]
        .concat(),
    )
}

fn positive_finite(value: f64) -> Option<f64> {
    let value = value.abs();
    (value.is_finite() && value > 0.0).then_some(value)
}

fn nonnegative_finite(value: f64) -> Option<f64> {
    let value = value.abs();
    value.is_finite().then_some(value)
}
