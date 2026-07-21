use std::panic::{AssertUnwindSafe, catch_unwind};

use dashu::float::{
    FBig,
    round::mode::{Down, Up},
};

fn panics(operation: impl FnOnce()) -> bool {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(operation)).is_err();
    std::panic::set_hook(hook);
    result
}

fn main() {
    // This is the finite f64 at Dashu's binary exponential range-reduction
    // boundary: rounding x / ln(2) reaches isize::MIN.
    let center = 0xc3d6_2e42_fefa_39efu64;
    let mut crash_count = 0usize;
    let mut center_up_p2_panics = false;
    for bits in center - 64..=center + 64 {
        let input = f64::from_bits(bits);
        for precision in [1, 2, 3, 8, 24, 53, 128, 256] {
            let up = FBig::<Up>::try_from(input)
                .unwrap()
                .with_precision(precision)
                .value();
            let down = FBig::<Down>::try_from(input)
                .unwrap()
                .with_precision(precision)
                .value();
            if panics(|| drop(up.exp_m1())) {
                crash_count += 1;
                center_up_p2_panics |= bits == center && precision == 2;
            }
            if panics(|| drop(down.exp_m1())) {
                crash_count += 1;
            }
        }
    }
    if cfg!(debug_assertions) {
        assert!(center_up_p2_panics);
        assert!(crash_count > 0);
    } else {
        assert_eq!(crash_count, 0);
    }
    println!(
        "DASHU-026 boundary sweep complete: profile={} cases=2064 panics={crash_count}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}
