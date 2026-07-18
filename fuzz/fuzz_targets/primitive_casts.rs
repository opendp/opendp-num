#![no_main]

use std::{cmp::Ordering, str::FromStr};

use libfuzzer_sys::fuzz_target;
use opendp_num_fuzz::{fail, signed_decimal, unsigned_decimal};

macro_rules! check_cast {
    ($source:expr, $ty:ty, $value:expr, $minimum:expr, $maximum:expr, $positive:expr, $input:expr, $exact_op:expr, $saturating_op:expr) => {{
        let expected_exact = if compare_decimal(&$value, &$minimum) != Ordering::Less
            && compare_decimal(&$value, &$maximum) != Ordering::Greater
        {
            Some($value.clone())
        } else {
            None
        };

        let actual_exact = <$ty>::try_from($source.clone())
            .ok()
            .map(|value| value.to_string());
        if actual_exact != expected_exact {
            fail(
                "primitive_casts",
                $exact_op,
                "exact big-integer cast acceptance or value differs from mathematical range",
                $input,
                &[
                    ("source", $value.clone()),
                    ("target", stringify!($ty).to_owned()),
                    ("minimum", $minimum.clone()),
                    ("maximum", $maximum.clone()),
                    ("expected", format!("{expected_exact:?}")),
                    ("actual", format!("{actual_exact:?}")),
                ],
            );
        }

        let actual_saturated = <$ty>::try_from($source.clone())
            .unwrap_or_else(|_| if $positive { <$ty>::MAX } else { <$ty>::MIN })
            .to_string();
        let expected_saturated = match compare_decimal(&$value, &$minimum) {
            Ordering::Less => $minimum.clone(),
            _ if compare_decimal(&$value, &$maximum) == Ordering::Greater => $maximum.clone(),
            _ => $value.clone(),
        };
        if actual_saturated != expected_saturated {
            fail(
                "primitive_casts",
                $saturating_op,
                "saturating big-integer cast does not clamp to the target range",
                $input,
                &[
                    ("source", $value.clone()),
                    ("target", stringify!($ty).to_owned()),
                    ("minimum", $minimum.clone()),
                    ("maximum", $maximum.clone()),
                    ("expected", expected_saturated),
                    ("actual", actual_saturated),
                ],
            );
        }
    }};
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let source_is_signed = data[0] & 1 == 0;
    let target = data[1] % 12;
    let selector = data[2];
    let arbitrary = if source_is_signed {
        signed_decimal(&data[3..], data[0] & 2 != 0, selector)
    } else {
        unsigned_decimal(&data[3..], selector)
    };

    macro_rules! dispatch {
        ($ty:ty) => {{
            let minimum = <$ty>::MIN.to_string();
            let maximum = <$ty>::MAX.to_string();
            let value = boundary_value(source_is_signed, selector, &minimum, &maximum, &arbitrary);
            if source_is_signed {
                let source = dashu::integer::IBig::from_str(&value).unwrap();
                let positive = !value.starts_with('-');
                check_cast!(
                    source,
                    $ty,
                    value,
                    minimum,
                    maximum,
                    positive,
                    data,
                    "integer_exact_to_primitive",
                    "integer_saturating_to_primitive"
                );
            } else {
                let source = dashu::integer::UBig::from_str(&value).unwrap();
                check_cast!(
                    source,
                    $ty,
                    value,
                    minimum,
                    maximum,
                    true,
                    data,
                    "natural_exact_to_primitive",
                    "natural_saturating_to_primitive"
                );
            }
        }};
    }

    match target {
        0 => dispatch!(i8),
        1 => dispatch!(i16),
        2 => dispatch!(i32),
        3 => dispatch!(i64),
        4 => dispatch!(i128),
        5 => dispatch!(isize),
        6 => dispatch!(u8),
        7 => dispatch!(u16),
        8 => dispatch!(u32),
        9 => dispatch!(u64),
        10 => dispatch!(u128),
        _ => dispatch!(usize),
    }
});

fn boundary_value(
    source_is_signed: bool,
    selector: u8,
    minimum: &str,
    maximum: &str,
    arbitrary: &str,
) -> String {
    match selector % 12 {
        0 => arbitrary.to_owned(),
        1 => if source_is_signed { minimum } else { "0" }.to_owned(),
        2 => maximum.to_owned(),
        3 => {
            if source_is_signed {
                decimal_offset(minimum, -1)
            } else {
                "0".to_owned()
            }
        }
        4 => decimal_offset(maximum, 1),
        5 => "0".to_owned(),
        6 => "1".to_owned(),
        7 => if source_is_signed { "-1" } else { "0" }.to_owned(),
        8 => decimal_offset(maximum, -1),
        9 => {
            if source_is_signed {
                decimal_offset(minimum, 1)
            } else {
                "1".to_owned()
            }
        }
        10 => power_of_two_near(maximum, false),
        _ => power_of_two_near(maximum, true),
    }
}

fn decimal_offset(value: &str, delta: i32) -> String {
    let mut value = rug::Integer::from_str(value).unwrap();
    value += delta;
    value.to_string()
}

fn power_of_two_near(maximum: &str, above: bool) -> String {
    let maximum = rug::Integer::from_str(maximum).unwrap();
    let bits = maximum.significant_bits();
    let mut value = rug::Integer::from(1);
    value <<= if above { bits } else { bits.saturating_sub(1) };
    value.to_string()
}

fn compare_decimal(lhs: &str, rhs: &str) -> Ordering {
    let (lhs_negative, lhs_magnitude) = split_sign(lhs);
    let (rhs_negative, rhs_magnitude) = split_sign(rhs);
    match (lhs_negative, rhs_negative) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => compare_magnitude(lhs_magnitude, rhs_magnitude),
        (true, true) => compare_magnitude(rhs_magnitude, lhs_magnitude),
    }
}

fn split_sign(value: &str) -> (bool, &str) {
    match value.strip_prefix('-') {
        Some(magnitude) if magnitude != "0" => (true, magnitude),
        _ => (false, value.trim_start_matches('+')),
    }
}

fn compare_magnitude(lhs: &str, rhs: &str) -> Ordering {
    let lhs = lhs.trim_start_matches('0');
    let rhs = rhs.trim_start_matches('0');
    let lhs = if lhs.is_empty() { "0" } else { lhs };
    let rhs = if rhs.is_empty() { "0" } else { rhs };
    lhs.len().cmp(&rhs.len()).then_with(|| lhs.cmp(rhs))
}
