use opendp_num::{Add, DirectedUnary, Direction, ExactBinary, Ln};
use proptest::prelude::*;

#[cfg(all(feature = "dashu", feature = "mpfr"))]
use opendp_num::DirectedPowI;

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

#[cfg(all(feature = "dashu", feature = "mpfr"))]
macro_rules! assert_extreme_unary_cases {
    ($ty:ty, $operation:ty, [$($input:expr),+ $(,)?]) => {{
        use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};

        for input in [$($input),+] {
            for direction in [Direction::Down, Direction::Nearest, Direction::Up] {
                let dashu = <Dashu as DirectedUnary<$operation, $ty>>::eval(input, direction);
                let mpfr = <Mpfr as DirectedUnary<$operation, $ty>>::eval(input, direction);
                match (dashu, mpfr) {
                    (Ok(dashu), Ok(mpfr)) => assert_eq!(
                        dashu.value.to_bits(),
                        mpfr.value.to_bits(),
                        "{}({input:?}) with {direction:?}",
                        stringify!($operation),
                    ),
                    (Err(dashu), Err(mpfr)) => assert_eq!(
                        dashu.kind,
                        mpfr.kind,
                        "{}({input:?}) with {direction:?}",
                        stringify!($operation),
                    ),
                    (dashu, mpfr) => panic!(
                        "{}({input:?}) with {direction:?}: Dashu={dashu:?}, MPFR={mpfr:?}",
                        stringify!($operation),
                    ),
                }
            }
        }
    }};
}

#[cfg(all(feature = "dashu", feature = "mpfr"))]
#[test]
fn dashu_extreme_transcendentals_match_mpfr_f64() {
    use opendp_num::{Exp, ExpM1, Ln1p, Sqrt};

    let next_above_negative_one = f64::from_bits((-1.0f64).to_bits() - 1);
    assert_extreme_unary_cases!(
        f64,
        Ln,
        [
            -f64::MAX,
            -f64::from_bits(1),
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            1.0,
            f64::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f64,
        Ln1p,
        [
            -f64::MAX,
            -1.0,
            next_above_negative_one,
            -f64::from_bits(1),
            -0.0,
            f64::from_bits(1),
            f64::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f64,
        Sqrt,
        [
            -f64::MAX,
            -f64::from_bits(1),
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            f64::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f64,
        Exp,
        [-f64::MAX, -746.0, -745.0, -0.0, 0.0, 709.0, 710.0, f64::MAX,]
    );
    assert_extreme_unary_cases!(
        f64,
        ExpM1,
        [-f64::MAX, -38.0, -37.0, -0.0, 0.0, 709.0, 710.0, f64::MAX,]
    );
}

#[cfg(all(feature = "dashu", feature = "mpfr"))]
#[test]
fn dashu_extreme_transcendentals_match_mpfr_f32() {
    use opendp_num::{Exp, ExpM1, Ln1p, Sqrt};

    let next_above_negative_one = f32::from_bits((-1.0f32).to_bits() - 1);
    assert_extreme_unary_cases!(
        f32,
        Ln,
        [
            -f32::MAX,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            1.0,
            f32::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f32,
        Ln1p,
        [
            -f32::MAX,
            -1.0,
            next_above_negative_one,
            -f32::from_bits(1),
            -0.0,
            f32::from_bits(1),
            f32::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f32,
        Sqrt,
        [
            -f32::MAX,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            f32::MAX,
        ]
    );
    assert_extreme_unary_cases!(
        f32,
        Exp,
        [-f32::MAX, -104.0, -103.0, -0.0, 0.0, 88.0, 89.0, f32::MAX,]
    );
    assert_extreme_unary_cases!(
        f32,
        ExpM1,
        [-f32::MAX, -18.0, -17.0, -0.0, 0.0, 88.0, 89.0, f32::MAX,]
    );
}

#[cfg(all(feature = "dashu", feature = "mpfr"))]
macro_rules! assert_extreme_powi_cases {
    ($ty:ty, [$($case:expr),+ $(,)?]) => {{
        use opendp_num::backend::{dashu::Dashu, mpfr::Mpfr};

        for (base, exponent) in [$($case),+] {
            for direction in [Direction::Down, Direction::Nearest, Direction::Up] {
                let dashu =
                    <Dashu as DirectedPowI<$ty>>::eval(base, &exponent, direction);
                let mpfr = <Mpfr as DirectedPowI<$ty>>::eval(base, &exponent, direction);
                match (dashu, mpfr) {
                    (Ok(dashu), Ok(mpfr)) => assert_eq!(
                        dashu.value.to_bits(),
                        mpfr.value.to_bits(),
                        "powi({base:?}, {exponent}) with {direction:?}",
                    ),
                    (Err(dashu), Err(mpfr)) => assert_eq!(
                        dashu.kind,
                        mpfr.kind,
                        "powi({base:?}, {exponent}) with {direction:?}",
                    ),
                    (dashu, mpfr) => panic!(
                        "powi({base:?}, {exponent}) with {direction:?}: Dashu={dashu:?}, MPFR={mpfr:?}",
                    ),
                }
            }
        }
    }};
}

#[cfg(all(feature = "dashu", feature = "mpfr"))]
#[test]
fn dashu_extreme_powi_matches_mpfr() {
    assert_extreme_powi_cases!(
        f64,
        [
            (f64::MAX, 2),
            (-f64::MAX, 3),
            (f64::MAX, -2),
            (f64::from_bits(1), 2),
            (-f64::from_bits(1), 3),
            (f64::from_bits(1), -2),
            (0.0, -1),
            (-0.0, -3),
            (1.0, i32::MAX),
            (-1.0, i32::MAX),
            (2.0, 2_000),
            (2.0, -2_000),
        ]
    );
    assert_extreme_powi_cases!(
        f32,
        [
            (f32::MAX, 2),
            (-f32::MAX, 3),
            (f32::MAX, -2),
            (f32::from_bits(1), 2),
            (-f32::from_bits(1), 3),
            (f32::from_bits(1), -2),
            (0.0, -1),
            (-0.0, -3),
            (1.0, i32::MAX),
            (-1.0, i32::MAX),
            (2.0, 200),
            (2.0, -200),
        ]
    );
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
