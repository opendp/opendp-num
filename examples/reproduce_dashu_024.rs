use std::panic::{AssertUnwindSafe, catch_unwind};

use dashu::{
    float::{FBig, round::mode::Up},
    integer::IBig,
};

fn panics(operation: impl FnOnce()) -> bool {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(operation)).is_err();
    std::panic::set_hook(hook);
    result
}

fn input(value: f64) -> FBig<Up> {
    FBig::<Up>::try_from(value)
        .unwrap()
        .with_precision(53)
        .value()
}

fn main() {
    let maximum = IBig::from(isize::MAX);
    let minimum = IBig::from(isize::MIN);

    // All three exact results are directly constructible finite FBig values.
    assert!(
        !FBig::<Up>::from_parts(IBig::ONE, isize::MAX)
            .repr()
            .is_infinite()
    );
    assert!(
        !FBig::<Up>::from_parts(-IBig::ONE, isize::MAX)
            .repr()
            .is_infinite()
    );
    assert!(
        !FBig::<Up>::from_parts(IBig::ONE, isize::MIN)
            .repr()
            .is_infinite()
    );

    assert!(panics(|| drop(input(2.0).powi(maximum.clone()))));
    assert!(panics(|| drop(input(-2.0).powi(maximum))));
    assert!(panics(|| drop(input(2.0).powi(minimum))));

    // The asymmetric lower-bound control succeeds.
    let control = input(0.5).powi(IBig::from(isize::MAX));
    assert_eq!(control.repr().exponent(), -isize::MAX);

    // One exponent later, the exact result is still representable at
    // isize::MIN, but the floating range guard reports underflow to zero.
    let lower_endpoint = input(0.5).powi(IBig::from(isize::MAX) + IBig::ONE);
    assert!(lower_endpoint.repr().significand().is_zero());
    let expected_lower_endpoint = FBig::<Up>::from_parts(IBig::ONE, isize::MIN);
    assert!(!expected_lower_endpoint.repr().significand().is_zero());

    println!(
        "DASHU-024 reproduced: powi panics or returns zero for exact representable isize boundary results"
    );
}
