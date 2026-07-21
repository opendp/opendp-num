use std::panic::{AssertUnwindSafe, catch_unwind};

use dashu::{
    base::Sign,
    float::{
        FBig,
        round::mode::{Down, Up},
    },
    integer::IBig,
};

fn panics(operation: impl FnOnce()) -> bool {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(operation)).is_err();
    std::panic::set_hook(hook);
    result
}

fn assert_exp_remains_positive<R: dashu::float::round::Round>() {
    for input in [-1_000.0, -10_000.0, -6.0e18] {
        let x = FBig::<R>::try_from(input)
            .unwrap()
            .with_precision(53)
            .value();
        let result = x.exp();
        assert!(!result.repr().significand().is_zero());
        assert_eq!(result.repr().sign(), Sign::Positive);
    }
}

fn assert_upward_expm1_stays_above_negative_one() {
    for precision in [1, 2, 3, 8, 24, 53] {
        for input in [-10.0, -37.0, -100.0, -1_000.0, -10_000.0] {
            let x = FBig::<Up>::try_from(input)
                .unwrap()
                .with_precision(precision)
                .value();
            assert!(x.exp_m1() > FBig::<Up>::NEG_ONE);
        }
    }
}

fn assert_astronomical_range_behavior() {
    // -2^63 is a finite, exactly represented input, but floor(x / ln(2)) does
    // not fit isize. Dashu explicitly saturates exp and exp_m1 at this point.
    let up = FBig::<Up>::from_parts(-IBig::ONE, 63);
    let down = FBig::<Down>::from_parts(-IBig::ONE, 63);
    assert_eq!(up.precision(), 1);
    let up_exp = up.exp();
    assert!(up_exp.repr().significand().is_zero());
    assert!(down.exp().repr().significand().is_zero());
    assert_eq!(up.exp_m1(), FBig::<Up>::NEG_ONE);
    assert_eq!(down.exp_m1(), FBig::<Down>::NEG_ONE);

    // Upward exp must return the minimum positive FBig, not zero. Upward
    // exp_m1 must return the adjacent precision-1 value above -1.
    let expected_exp_up = FBig::<Up>::from_parts(IBig::ONE, isize::MIN);
    let expected_expm1_up = FBig::<Up>::from_parts(-IBig::ONE, -1);
    assert!(expected_exp_up > up_exp);
    assert_ne!(up.exp_m1(), expected_expm1_up);
}

fn assert_raw_exp_literal_extremes() {
    let max_up = FBig::<Up>::try_from(f64::MAX)
        .unwrap()
        .with_precision(53)
        .value();
    let max_down = FBig::<Down>::try_from(f64::MAX)
        .unwrap()
        .with_precision(53)
        .value();
    let min_up = FBig::<Up>::try_from(-f64::MAX)
        .unwrap()
        .with_precision(53)
        .value();
    let min_down = FBig::<Down>::try_from(-f64::MAX)
        .unwrap()
        .with_precision(53)
        .value();
    assert!(max_up.exp().repr().is_infinite());
    assert!(max_down.exp().repr().is_infinite());
    assert!(min_up.exp().repr().significand().is_zero());
    assert!(min_down.exp().repr().significand().is_zero());
    let expected_up = FBig::<Up>::from_parts(IBig::ONE, isize::MIN)
        .with_precision(53)
        .value();
    assert!(expected_up > min_up.exp());

    let max32_up = FBig::<Up>::try_from(f32::MAX)
        .unwrap()
        .with_precision(24)
        .value();
    let max32_down = FBig::<Down>::try_from(f32::MAX)
        .unwrap()
        .with_precision(24)
        .value();
    let min32_up = FBig::<Up>::try_from(-f32::MAX)
        .unwrap()
        .with_precision(24)
        .value();
    let min32_down = FBig::<Down>::try_from(-f32::MAX)
        .unwrap()
        .with_precision(24)
        .value();
    assert!(max32_up.exp().repr().is_infinite());
    assert!(max32_down.exp().repr().is_infinite());
    assert!(min32_up.exp().repr().significand().is_zero());
    assert!(min32_down.exp().repr().significand().is_zero());
    let expected32_up = FBig::<Up>::from_parts(IBig::ONE, isize::MIN)
        .with_precision(24)
        .value();
    assert!(expected32_up > min32_up.exp());
}

fn assert_exact_power<R: dashu::float::round::Round>(
    base: f64,
    exponent: IBig,
    expected_significand: i8,
    expected_exponent: isize,
) {
    let base = FBig::<R>::try_from(base)
        .unwrap()
        .with_precision(53)
        .value();
    let result = base.powi(exponent);
    assert_eq!(
        result.repr().significand(),
        &IBig::from(expected_significand)
    );
    assert_eq!(result.repr().exponent(), expected_exponent);
    assert!(!result.repr().is_infinite());
}

fn audit_power_mode<R: dashu::float::round::Round>() {
    let even = IBig::from(10u64).pow(16);
    let odd = &even + IBig::ONE;
    let even_isize = isize::try_from(&even).unwrap();
    let odd_isize = isize::try_from(&odd).unwrap();

    assert_exact_power::<R>(0.5, even.clone(), 1, -even_isize);
    assert_exact_power::<R>(2.0, even.clone(), 1, even_isize);
    assert_exact_power::<R>(-0.5, odd, -1, -odd_isize);
    assert_exact_power::<R>(2.0, -even, 1, -even_isize);

    // Exercise the exact representable FBig exponent boundaries directly.
    let max_exponent = IBig::from(isize::MAX);
    let min_exponent = IBig::from(isize::MIN);
    let boundary_panics = [
        panics(|| assert_exact_power::<R>(2.0, max_exponent.clone(), 1, isize::MAX)),
        panics(|| assert_exact_power::<R>(0.5, max_exponent.clone(), 1, -isize::MAX)),
        panics(|| assert_exact_power::<R>(-2.0, max_exponent, -1, isize::MAX)),
        panics(|| assert_exact_power::<R>(2.0, min_exponent, 1, isize::MIN)),
    ];
    assert_eq!(boundary_panics, [true, false, true, true]);
}

fn assert_raw_powi_literal_extremes<R: dashu::float::round::Round>() {
    let cases = [
        (f64::MAX, IBig::from(2), Sign::Positive),
        (-f64::MAX, IBig::from(3), Sign::Negative),
        (f64::MAX, IBig::from(-2), Sign::Positive),
        (f64::from_bits(1), IBig::from(2), Sign::Positive),
        (-f64::from_bits(1), IBig::from(3), Sign::Negative),
        (f64::from_bits(1), IBig::from(-2), Sign::Positive),
        (f64::MAX, IBig::from(i32::MAX), Sign::Positive),
        (f64::from_bits(1), IBig::from(i32::MAX), Sign::Positive),
    ];

    for (base, exponent, expected_sign) in cases {
        let base = FBig::<R>::try_from(base)
            .unwrap()
            .with_precision(53)
            .value();
        let result = base.powi(exponent);
        assert!(!result.repr().is_infinite());
        assert!(!result.repr().significand().is_zero());
        assert_eq!(result.repr().sign(), expected_sign);
    }

    let cases32 = [
        (f32::MAX, IBig::from(2), Sign::Positive),
        (-f32::MAX, IBig::from(3), Sign::Negative),
        (f32::MAX, IBig::from(-2), Sign::Positive),
        (f32::from_bits(1), IBig::from(2), Sign::Positive),
        (-f32::from_bits(1), IBig::from(3), Sign::Negative),
        (f32::from_bits(1), IBig::from(-2), Sign::Positive),
        (f32::MAX, IBig::from(i32::MAX), Sign::Positive),
        (f32::from_bits(1), IBig::from(i32::MAX), Sign::Positive),
    ];

    for (base, exponent, expected_sign) in cases32 {
        let base = FBig::<R>::try_from(base)
            .unwrap()
            .with_precision(24)
            .value();
        let result = base.powi(exponent);
        assert!(!result.repr().is_infinite());
        assert!(!result.repr().significand().is_zero());
        assert_eq!(result.repr().sign(), expected_sign);
    }
}

fn assert_powi_out_of_range_behavior() {
    let overflow_exponent = IBig::from(isize::MAX) + IBig::ONE;
    let underflow_exponent = &overflow_exponent + IBig::ONE;
    let down_two = FBig::<Down>::from_parts(IBig::ONE, 1);
    let down_half = FBig::<Down>::from_parts(IBig::ONE, -1);

    // Downward positive overflow should be the largest finite precision-1
    // value, but the convenience API returns infinity in both directions.
    assert!(down_two.powi(overflow_exponent).repr().is_infinite());
    assert!(
        !FBig::<Down>::from_parts(IBig::ONE, isize::MAX)
            .repr()
            .is_infinite()
    );

    // The negative-exponent reciprocal path panics after its intermediate
    // positive power saturates to infinity.
    assert!(panics(|| drop(down_half.powi(-underflow_exponent))));
}

fn assert_exp_m1_range_boundary_profile() {
    let input = f64::from_bits(0xc3d6_2e42_fefa_39ef);
    let up = FBig::<Up>::try_from(input)
        .unwrap()
        .with_precision(2)
        .value();
    assert_eq!(panics(|| drop(up.exp_m1())), cfg!(debug_assertions));
}

fn main() {
    let zero = FBig::<Up>::try_from(0.0f64).unwrap();
    assert_eq!(zero.precision(), 0);
    assert!(panics(|| drop(zero.exp())));

    // Nonzero exact primitive conversions carry the primitive mantissa width.
    for value in [0.5, 1.0, 2.0, 4.0] {
        assert_eq!(FBig::<Up>::try_from(value).unwrap().precision(), 53);
    }
    // The exact public constant is unlimited and hits the same operation precondition.
    assert_eq!(FBig::<Up>::ONE.precision(), 0);
    assert!(panics(|| drop(FBig::<Up>::ONE.ln())));

    assert_exp_remains_positive::<Up>();
    assert_exp_remains_positive::<Down>();
    assert_upward_expm1_stays_above_negative_one();
    assert_astronomical_range_behavior();
    assert_raw_exp_literal_extremes();
    audit_power_mode::<Up>();
    audit_power_mode::<Down>();
    assert_raw_powi_literal_extremes::<Up>();
    assert_raw_powi_literal_extremes::<Down>();
    assert_powi_out_of_range_behavior();
    assert_exp_m1_range_boundary_profile();

    println!(
        "PR2801 audit: DASHU-022 through DASHU-026 reproduced across directed range boundaries"
    );
}
