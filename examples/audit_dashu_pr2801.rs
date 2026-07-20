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
    audit_power_mode::<Up>();
    audit_power_mode::<Down>();

    println!(
        "PR2801 audit: precision-state issue reproduced; exp, exp_m1, and powi saturation candidates did not reproduce in raw FBig"
    );
}
