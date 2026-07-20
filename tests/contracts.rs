use opendp_num::{Add, DirectedPowI, DirectedUnary, Direction, ExactBinary, Ln};
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

#[cfg(all(feature = "dashu", feature = "mpfr"))]
#[test]
fn arbitrary_precision_power_exponents_are_not_narrowed() {
    use dashu::integer::IBig;
    use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};
    use rug::Integer;

    let dashu_odd = (IBig::from(1u8) << 200) + IBig::ONE;
    let mpfr_odd = (Integer::from(1u8) << 200) + 1u8;
    let dashu =
        <Dashu as DirectedPowI<f64, IBig>>::eval(-1.0, &dashu_odd, Direction::Nearest).unwrap();
    let mpfr =
        <Mpfr as DirectedPowI<f64, Integer>>::eval(-1.0, &mpfr_odd, Direction::Nearest).unwrap();
    assert_eq!(dashu.value.to_bits(), (-1.0f64).to_bits());
    assert_eq!(dashu.value.to_bits(), mpfr.value.to_bits());

    let dashu_negative = -(IBig::from(1u8) << 200);
    let mut mpfr_negative = Integer::from(1u8) << 200usize;
    mpfr_negative = -mpfr_negative;
    let dashu_up =
        <Dashu as DirectedPowI<f64, IBig>>::eval(2.0, &dashu_negative, Direction::Up).unwrap();
    let mpfr_up =
        <Mpfr as DirectedPowI<f64, Integer>>::eval(2.0, &mpfr_negative, Direction::Up).unwrap();
    assert_eq!(dashu_up.value.to_bits(), 1);
    assert_eq!(dashu_up.value.to_bits(), mpfr_up.value.to_bits());
}

#[cfg(all(feature = "malachite", feature = "mpfr"))]
#[test]
fn negative_power_of_signed_zero_is_division_by_zero() {
    use opendp_num::{
        ErrorKind,
        backend::{malachite::Malachite, mpfr::Mpfr},
    };

    let exponent = -1;
    let malachite =
        <Malachite as DirectedPowI<f32>>::eval(-0.0, &exponent, Direction::Down).unwrap_err();
    let mpfr = <Mpfr as DirectedPowI<f32>>::eval(-0.0, &exponent, Direction::Down).unwrap_err();
    assert_eq!(malachite.kind, ErrorKind::DivisionByZero);
    assert_eq!(malachite.kind, mpfr.kind);
}
