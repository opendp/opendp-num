use dashu::{
    base::Approximation,
    float::{
        FBig,
        round::mode::{Down, Up},
    },
    integer::IBig,
};

fn main() {
    // x = -2^63 is finite and exactly represented with one bit of precision.
    let up_input = FBig::<Up>::from_parts(-IBig::ONE, 63);
    let down_input = FBig::<Down>::from_parts(-IBig::ONE, 63);
    assert_eq!(up_input.precision(), 1);

    let up_approximation = up_input
        .context()
        .exp_m1(up_input.repr(), None)
        .expect("finite exp_m1 input");
    let down_approximation = down_input
        .context()
        .exp_m1(down_input.repr(), None)
        .expect("finite exp_m1 input");

    assert!(matches!(
        up_approximation,
        Approximation::Exact(ref value) if value == &FBig::<Up>::NEG_ONE
    ));
    assert!(matches!(
        down_approximation,
        Approximation::Exact(ref value) if value == &FBig::<Down>::NEG_ONE
    ));

    // Since 0 < exp(-2^63) < 1/2, the exact result lies strictly between
    // -1 and -1/2. At precision 1, Up must therefore return -1/2.
    let expected_up = FBig::<Up>::from_parts(-IBig::ONE, -1);
    assert_eq!(expected_up.to_f64().value(), -0.5);

    println!("DASHU-023 reproduced: Up exp_m1(-2^63)=-1 Exact; expected=-0.5 at precision=1");
}
