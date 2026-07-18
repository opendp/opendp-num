use opendp_num::{Add, DirectedUnary, Direction, ExactBinary, Ln};
use proptest::prelude::*;

proptest! {
    #[test]
    fn integer_addition_agrees(lhs: i64, rhs: i64) {
        let expected = i128::from(lhs) + i128::from(rhs);

        #[cfg(feature = "dashu")]
        {
            use dashu::integer::IBig;
            use opendp_num::backend::dashu::Dashu;
            prop_assert_eq!(<Dashu as ExactBinary<Add, IBig>>::eval(&IBig::from(lhs), &IBig::from(rhs)), IBig::from(expected));
        }
        #[cfg(feature = "malachite")]
        {
            use malachite::Integer;
            use opendp_num::backend::malachite::Malachite;
            prop_assert_eq!(<Malachite as ExactBinary<Add, Integer>>::eval(&Integer::from(lhs), &Integer::from(rhs)), Integer::from(expected));
        }
        #[cfg(feature = "mpfr")]
        {
            use rug::Integer;
            use opendp_num::backend::mpfr::Mpfr;
            prop_assert_eq!(<Mpfr as ExactBinary<Add, Integer>>::eval(&Integer::from(lhs), &Integer::from(rhs)), Integer::from(expected));
        }
    }
}

#[cfg(feature = "dashu")]
#[test]
fn dashu_directed_log_encloses_std_result() {
    use opendp_num::backend::dashu::Dashu;
    let down = <Dashu as DirectedUnary<Ln, f64>>::eval(1.25, Direction::Down)
        .unwrap()
        .value;
    let up = <Dashu as DirectedUnary<Ln, f64>>::eval(1.25, Direction::Up)
        .unwrap()
        .value;
    let nearest = 1.25f64.ln();
    assert!(down <= nearest);
    assert!(up >= nearest);
}
