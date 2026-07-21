use std::panic::{AssertUnwindSafe, catch_unwind};

use dashu::{
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

fn main() {
    let overflow_exponent = IBig::from(isize::MAX) + IBig::ONE;
    let underflow_exponent = &overflow_exponent + IBig::ONE;

    let up_two = FBig::<Up>::from_parts(IBig::ONE, 1);
    let down_two = FBig::<Down>::from_parts(IBig::ONE, 1);
    let down_half = FBig::<Down>::from_parts(IBig::ONE, -1);

    let up_overflow = up_two.powi(overflow_exponent.clone());
    let down_overflow = down_two.powi(overflow_exponent);
    assert!(up_overflow.repr().is_infinite());
    assert!(down_overflow.repr().is_infinite());

    // Downward positive overflow must be the largest finite precision-1 FBig.
    let expected_down = FBig::<Down>::from_parts(IBig::ONE, isize::MAX);
    assert!(!expected_down.repr().is_infinite());

    // The negative-exponent reciprocal path can panic before returning its
    // directed endpoint once the exact magnitude is below the exponent range.
    assert!(panics(|| {
        drop(down_half.powi(-underflow_exponent.clone()))
    }));

    println!(
        "DASHU-025 reproduced: powi discards directed overflow endpoints and panics on out-of-range reciprocal"
    );
}
