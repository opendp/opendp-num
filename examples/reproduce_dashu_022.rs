use std::panic::{AssertUnwindSafe, catch_unwind};

use dashu::float::{FBig, round::mode::Up};

fn panics(operation: impl FnOnce()) -> bool {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(operation)).is_err();
    std::panic::set_hook(hook);
    result
}

fn main() {
    let zero = FBig::<Up>::try_from(0.0f64).expect("zero is a valid finite input");
    assert_eq!(zero.precision(), 0);

    let outcomes = [
        ("exp", panics(|| drop(zero.exp()))),
        ("exp_m1", panics(|| drop(zero.exp_m1()))),
        ("sqrt", panics(|| drop(zero.sqrt()))),
        ("ln_1p", panics(|| drop(zero.ln_1p()))),
    ];
    assert!(outcomes.iter().all(|(_, did_panic)| *did_panic));

    println!(
        "DASHU-022 reproduced: try_from(0.0) precision=0; {} panic",
        outcomes
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ")
    );
}
