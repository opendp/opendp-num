use arbitrary::Arbitrary;
use opendp_num::Direction;
use serde_json::{Map, Value, json};
use std::{
    any::Any,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Arbitrary, Clone, Debug)]
pub struct UnaryCase {
    pub format: u8,
    pub operation: u8,
    pub direction: u8,
    pub selector: u8,
    pub bits: u64,
    pub exponent: i32,
}

#[derive(Arbitrary, Clone, Debug)]
pub struct BinaryCase {
    pub format: u8,
    pub operation: u8,
    pub direction: u8,
    pub lhs_selector: u8,
    pub rhs_selector: u8,
    pub lhs_bits: u64,
    pub rhs_bits: u64,
}

#[derive(Arbitrary, Clone, Debug)]
pub struct ConversionCase {
    pub operation: u8,
    pub direction: u8,
    pub selector: u8,
    pub bits: u64,
    pub sign: bool,
    pub payload: Vec<u8>,
}

pub fn directed_direction(value: u8) -> Direction {
    if value & 1 == 0 {
        Direction::Down
    } else {
        Direction::Up
    }
}

pub fn any_direction(value: u8) -> Direction {
    match value % 3 {
        0 => Direction::Down,
        1 => Direction::Nearest,
        _ => Direction::Up,
    }
}

pub fn special_exponent(selector: u8, raw: i32) -> i32 {
    const SPECIAL: &[i32] = &[
        i32::MIN,
        -4096,
        -1024,
        -256,
        -128,
        -64,
        -54,
        -53,
        -32,
        -2,
        -1,
        0,
        1,
        2,
        24,
        32,
        53,
        64,
        127,
        128,
        256,
        1024,
        4096,
        i32::MAX,
    ];
    if selector & 3 != 0 {
        SPECIAL[selector as usize % SPECIAL.len()]
    } else {
        raw.clamp(-4096, 4096)
    }
}

pub fn special_f64(selector: u8, bits: u64) -> f64 {
    const SPECIAL: &[u64] = &[
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        0x800f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x8010_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0x3fef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0001,
        0xbfefffff_ffffffff,
    ];
    match selector % 32 {
        0..=19 => f64::from_bits(SPECIAL[selector as usize % SPECIAL.len()]),
        20 => f64::from_bits(1.0f64.to_bits().wrapping_add(bits & 0xff)),
        21 => f64::from_bits(1.0f64.to_bits().wrapping_sub(bits & 0xff)),
        22 => f64::from_bits((-1.0f64).to_bits().wrapping_add(bits & 0xff)),
        23 => f64::from_bits((-1.0f64).to_bits().wrapping_sub(bits & 0xff)),
        24 => 709.0 + ((bits as i16) as f64) * f64::EPSILON,
        25 => -745.0 + ((bits as i16) as f64) * f64::EPSILON,
        26 => f64::from_bits(((bits % 2046) << 52) | 1),
        27 => -f64::from_bits(((bits % 2046) << 52) | 1),
        _ => f64::from_bits(bits),
    }
}

pub fn special_f32(selector: u8, bits: u64) -> f32 {
    let bits = bits as u32;
    const SPECIAL: &[u32] = &[
        0x0000_0000,
        0x8000_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x4000_0000,
        0x3f00_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x807f_ffff,
        0x0080_0000,
        0x8080_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0x3f7f_ffff,
        0x3f80_0001,
        0xbf7f_ffff,
    ];
    match selector % 32 {
        0..=19 => f32::from_bits(SPECIAL[selector as usize % SPECIAL.len()]),
        20 => f32::from_bits(1.0f32.to_bits().wrapping_add(bits & 0x3f)),
        21 => f32::from_bits(1.0f32.to_bits().wrapping_sub(bits & 0x3f)),
        22 => f32::from_bits((-1.0f32).to_bits().wrapping_add(bits & 0x3f)),
        23 => f32::from_bits((-1.0f32).to_bits().wrapping_sub(bits & 0x3f)),
        24 => 88.0 + ((bits as i8) as f32) * f32::EPSILON,
        25 => -103.0 + ((bits as i8) as f32) * f32::EPSILON,
        26 => f32::from_bits(((bits % 254) << 23) | 1),
        27 => -f32::from_bits(((bits % 254) << 23) | 1),
        _ => f32::from_bits(bits),
    }
}

/// Convert unsigned big-endian bytes to decimal without trusting any backend.
pub fn unsigned_decimal(bytes: &[u8], selector: u8) -> String {
    if selector % 16 == 1 {
        return "0".to_owned();
    }
    if selector % 16 == 2 {
        return "1".to_owned();
    }
    if selector % 16 == 3 {
        let exponent = bytes.first().copied().unwrap_or_default() as usize
            + bytes.get(1).copied().unwrap_or_default() as usize * 256;
        return power_of_two_decimal(exponent.min(8192));
    }

    let bytes = if bytes.len() > 4096 {
        &bytes[..4096]
    } else {
        bytes
    };
    let mut limbs = vec![0u32];
    for &byte in bytes {
        let mut carry = u64::from(byte);
        for limb in &mut limbs {
            let value = u64::from(*limb) * 256 + carry;
            *limb = (value % 1_000_000_000) as u32;
            carry = value / 1_000_000_000;
        }
        if carry != 0 {
            limbs.push(carry as u32);
        }
    }
    while limbs.len() > 1 && limbs.last() == Some(&0) {
        limbs.pop();
    }
    let mut output = limbs.last().copied().unwrap_or_default().to_string();
    for limb in limbs.iter().rev().skip(1) {
        use std::fmt::Write;
        let _ = write!(output, "{limb:09}");
    }
    output
}

pub fn signed_decimal(bytes: &[u8], negative: bool, selector: u8) -> String {
    let magnitude = unsigned_decimal(bytes, selector);
    if negative && magnitude != "0" {
        format!("-{magnitude}")
    } else {
        magnitude
    }
}

fn power_of_two_decimal(exponent: usize) -> String {
    let mut limbs = vec![1u32];
    for _ in 0..exponent {
        let mut carry = 0u64;
        for limb in &mut limbs {
            let value = u64::from(*limb) * 2 + carry;
            *limb = (value % 1_000_000_000) as u32;
            carry = value / 1_000_000_000;
        }
        if carry != 0 {
            limbs.push(carry as u32);
        }
    }
    let mut output = limbs.last().copied().unwrap_or_default().to_string();
    for limb in limbs.iter().rev().skip(1) {
        use std::fmt::Write;
        let _ = write!(output, "{limb:09}");
    }
    output
}

pub fn split_evenly(data: &[u8], count: usize) -> Vec<&[u8]> {
    if count == 0 {
        return Vec::new();
    }
    (0..count)
        .map(|index| {
            let start = data.len() * index / count;
            let end = data.len() * (index + 1) / count;
            &data[start..end]
        })
        .collect()
}

pub fn catch_backend<T>(
    target: &str,
    operation: &str,
    input: &[u8],
    fields: &[(&str, String)],
    f: impl FnOnce() -> T,
) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => fail(
            target,
            operation,
            "backend panic",
            input,
            &[fields, &[("panic", panic_message(payload.as_ref()))]].concat(),
        ),
    }
}

pub fn fail(
    target: &str,
    operation: &str,
    reason: &str,
    input: &[u8],
    fields: &[(&str, String)],
) -> ! {
    let report_root = std::env::var_os("OPENDP_NUM_FUZZ_REPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fuzz/reports"));
    let target_dir = report_root.join(target);
    let _ = fs::create_dir_all(&target_dir);

    let mut hash_material = Vec::new();
    hash_material.extend_from_slice(target.as_bytes());
    hash_material.extend_from_slice(operation.as_bytes());
    hash_material.extend_from_slice(reason.as_bytes());
    hash_material.extend_from_slice(input);
    let id = format!("{:016x}", fnv1a64(&hash_material));

    let raw_path = target_dir.join(format!("{id}.input"));
    let json_path = target_dir.join(format!("{id}.json"));
    let _ = fs::write(&raw_path, input);

    let mut context = Map::new();
    for (key, value) in fields {
        context.insert((*key).to_owned(), Value::String(value.clone()));
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let report = json!({
        "schema": 1,
        "timestamp_unix": timestamp,
        "target": target,
        "operation": operation,
        "reason": reason,
        "reproducer": raw_path,
        "input_hex": hex(input),
        "context": context,
    });
    if let Ok(serialized) = serde_json::to_vec_pretty(&report) {
        let temporary = json_path.with_extension("json.tmp");
        if fs::write(&temporary, serialized).is_ok() {
            let _ = fs::rename(temporary, &json_path);
        }
    }

    eprintln!(
        "OPENDP_NUM_VIOLATION target={target} operation={operation} reason={reason} report={}",
        json_path.display()
    );
    panic!("opendp-num fuzz violation: {target}/{operation}: {reason}");
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}
