#!/usr/bin/env python3
"""Create deterministic high-value seeds and libFuzzer dictionaries."""

from __future__ import annotations

import argparse
import struct
import hashlib
from pathlib import Path

from quarantine_known import known_input_hashes

ROOT = Path(__file__).resolve().parent
CORPUS_ROOT = ROOT / "corpus"
KNOWN_HASHES = known_input_hashes()


def write_seed(target: str, name: str, data: bytes) -> None:
    if hashlib.sha256(data).hexdigest() in KNOWN_HASHES.get(target, set()):
        return
    directory = CORPUS_ROOT / target
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / name
    if not path.exists() or path.read_bytes() != data:
        path.write_bytes(data)


def unary_case(fmt: int, op: int, direction: int, selector: int, bits: int, exponent: int = 1) -> bytes:
    return struct.pack("<BBBBQi", fmt, op, direction, selector, bits, exponent)


def binary_case(
    fmt: int,
    op: int,
    direction: int,
    lhs_selector: int,
    rhs_selector: int,
    lhs_bits: int,
    rhs_bits: int,
) -> bytes:
    return struct.pack("<BBBBBQQ", fmt, op, direction, lhs_selector, rhs_selector, lhs_bits, rhs_bits)


def backend_conversion_case(
    fmt: int,
    radix: int,
    rounding: int,
    negative: bool,
    exponent: int,
    significand: int,
) -> bytes:
    magnitude = abs(significand)
    payload = magnitude.to_bytes(max(1, (magnitude.bit_length() + 7) // 8), "big")
    # Vec<u8>::arbitrary encodes each item as `true, value`, terminated by
    # `false`; this explicit form keeps deterministic mathematical seeds exact.
    encoded_payload = b"".join(b"\x01" + bytes([byte]) for byte in payload) + b"\x00"
    return struct.pack("<BBBBi", fmt, radix, rounding, negative, exponent) + encoded_payload


def backend_extreme_case(
    operation: int,
    precision: int,
    input_selector: int,
    bits: int,
    base_exponent: int,
    exponent: int,
    offset: int,
    negative: bool,
) -> bytes:
    return struct.pack(
        "<BBBQBBhB",
        operation,
        precision,
        input_selector,
        bits,
        base_exponent,
        exponent,
        offset,
        negative,
    )


def main() -> None:
    global CORPUS_ROOT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus-root",
        type=Path,
        help="write corpora here instead of fuzz/corpus (dictionaries remain in fuzz/dictionaries)",
    )
    args = parser.parse_args()
    if args.corpus_root is not None:
        CORPUS_ROOT = args.corpus_root.resolve()

    float_bits = {
        "zero": 0x0000000000000000,
        "neg_zero": 0x8000000000000000,
        "one": 0x3FF0000000000000,
        "neg_one": 0xBFF0000000000000,
        "min_subnormal": 0x0000000000000001,
        "min_normal": 0x0010000000000000,
        "max": 0x7FEFFFFFFFFFFFFF,
        "inf": 0x7FF0000000000000,
        "nan": 0x7FF8000000000000,
        "exp_overflow_edge": struct.unpack("<Q", struct.pack("<d", 709.782712893384))[0],
        "exp_underflow_edge": struct.unpack("<Q", struct.pack("<d", -745.1332191019411))[0],
        "minus_one_next_up": 0xBFEFFFFFFFFFFFFF,
    }

    for op in range(7):
        for index, (name, bits) in enumerate(float_bits.items()):
            write_seed("directed_unary", f"op{op}-{name}", unary_case(0, op, index & 1, 31, bits, 2))
            write_seed("directed_unary", f"f32-op{op}-{name}", unary_case(1, op, index & 1, 31, bits, -2))

    # Raw Dashu extrema: every precision, literal primitive boundary, exact
    # exponent endpoint and immediate neighbor is represented deterministically.
    raw_exp_values = {
        "zero": 0.0,
        "neg_zero": -0.0,
        "one": 1.0,
        "neg_one": -1.0,
        "f64_max": float.fromhex("0x1.fffffffffffffp+1023"),
        "neg_f64_max": -float.fromhex("0x1.fffffffffffffp+1023"),
        "range_reduction_edge": -(2**63) * 0.6931471805599453,
        "neg_two_to_63": float(-(2**63)),
    }
    for operation in (0, 1):
        for precision in range(8):
            for name, value in raw_exp_values.items():
                bits = struct.unpack("<Q", struct.pack("<d", value))[0]
                write_seed(
                    "backend_float_extremes",
                    f"op{operation}-p{precision}-{name}",
                    backend_extreme_case(operation, precision, 31, bits, 0, 0, 0, False),
                )

    for precision in range(8):
        for base_selector in range(9):
            for exponent_selector in range(8):
                for offset in (-2, -1, 0, 1, 2):
                    for negative in (False, True):
                        write_seed(
                            "backend_float_extremes",
                            (
                                f"powi-p{precision}-b{base_selector}-e{exponent_selector}"
                                f"-o{offset}-n{int(negative)}"
                            ),
                            backend_extreme_case(
                                2,
                                precision,
                                0,
                                0,
                                base_selector,
                                exponent_selector,
                                offset,
                                negative,
                            ),
                        )

    # Exercise arbitrary-precision exponent parsing, sign, and parity beyond i32.
    for name, sign_parity, magnitude in [
        ("huge-positive-even", 0, 1 << 255),
        ("huge-positive-odd", 2, (1 << 255) + 1),
        ("huge-negative-even", 1, 1 << 255),
        ("huge-negative-odd", 3, (1 << 255) + 1),
    ]:
        tail = bytes([sign_parity]) + magnitude.to_bytes(32, "big")
        write_seed("directed_unary", name, unary_case(0, 6, 0, 0, float_bits["one"], 1) + tail)

    # Dashu PR #91: subnormal halfways nudged on both sides. Include
    # binary and mathematically equivalent decimal constructions.
    for fmt, boundary, m_values, j_values in [
        (0, 1075, [3, 21, (1 << 8) + 5, (1 << 20) + 3, (1 << 33) + 7, (1 << 40) + 9], [54, 60, 64, 96]),
        (1, 150, [3, 21, (1 << 10) + 5, (1 << 18) + 7], [25, 30, 32, 48]),
    ]:
        for m in m_values:
            for j in j_values:
                scale = boundary + j
                core = (2 * m + 1) << j
                for delta, side in [(1, "above"), (-1, "below")]:
                    sig = core + delta
                    stem = f"pr91-f{64 if fmt == 0 else 32}-m{m}-j{j}-{side}"
                    write_seed(
                        "backend_float_conversion",
                        stem + "-binary",
                        backend_conversion_case(fmt, 0, 0, False, -scale, sig),
                    )
                    write_seed(
                        "backend_float_conversion",
                        stem + "-decimal",
                        backend_conversion_case(fmt, 1, 0, False, -scale, sig * (5 ** scale)),
                    )

    # High-precision decimal inputs that panicked in debug builds before PR #91.
    for index, (significand, exponent) in enumerate([
        (1234567890123456789012345678901, -13),
        (3915263378237002511617337316730, -19),
        (1234567890123456789012345678901, -5),
        (9999999999999999999999999999999, -3),
        (27182818284590452353602874713526, -13),
    ]):
        write_seed(
            "backend_float_conversion",
            f"pr91-high-precision-decimal-{index}",
            backend_conversion_case(0, 1, 0, False, exponent, significand),
        )

    binary_values = list(float_bits.items())[:9]
    for op in range(4):
        for index, (name, bits) in enumerate(binary_values):
            write_seed(
                "directed_binary",
                f"op{op}-{name}",
                binary_case(0, op, index & 1, 31, 31, bits, float_bits["one"]),
            )

    exact_patterns = (
        b"\x00" * 16,
        b"\xff" * 64,
        bytes(range(256)),
        b"\x80" + b"\x00" * 255,
        b"\x01" + b"\x00" * 1023,
    )
    for index, pattern in enumerate(exact_patterns):
        write_seed("exact_integer", f"pattern-{index}", bytes([index % 12, 0, 3, 4, 0, 1, 0]) + pattern)
        write_seed("exact_rational", f"pattern-{index}", bytes([index % 11, 0, 3, 4, 5, 0, 1]) + pattern)
        write_seed("conversions", f"pattern-{index}", bytes([index % 9, index % 3, 31, 0]) + pattern)
        write_seed("primitive_casts", f"pattern-{index}", bytes([index & 1, index % 12, index % 12]) + pattern)

    write_seed("exact_rational", "round-positive-half", bytes([9, 0, 0, 0, 0, 0, 0, 1, 0, 2, 1]))
    write_seed("exact_rational", "round-negative-half", bytes([9, 0, 0, 0, 0, 1, 0, 1, 0, 2, 1]))

    for target_type in range(12):
        for selector in range(1, 12):
            write_seed(
                "primitive_casts",
                f"type{target_type}-boundary{selector}",
                bytes([0, target_type, selector, 0xff, 0x00, 0x01]),
            )
            write_seed(
                "primitive_casts",
                f"natural-type{target_type}-boundary{selector}",
                bytes([1, target_type, selector, 0xff, 0x00, 0x01]),
            )

    for op in range(5):
        alp = bytes([op, 31, 31]) + struct.pack(
            "<HQQQ",
            [1, 24, 53, 64, 128][op],
            float_bits["one"],
            struct.unpack("<Q", struct.pack("<d", 4.0))[0],
            [0, 1, 2, (1 << 52) - 1, (1 << 64) - 1][op],
        )
        write_seed("alp_primitives", f"op{op}-boundary", alp)

    sequence = bytearray([31])
    sequence.extend(struct.pack("<Q", float_bits["one"]))
    for op in range(11):
        sequence.extend(bytes([op, 31]))
        sequence.extend(struct.pack("<Q", float_bits["one"]))
    write_seed("opendp_sequences", "all-operations", bytes(sequence))

    dictionaries = ROOT / "dictionaries"
    dictionaries.mkdir(parents=True, exist_ok=True)
    (dictionaries / "float.dict").write_text(
        "\n".join(
            f'{name}="' + "".join(f"\\x{byte:02x}" for byte in struct.pack("<Q", bits)) + '"'
            for name, bits in float_bits.items()
        )
        + "\n"
    )
    (dictionaries / "integer.dict").write_text(
        '\n'.join([
            'zero="\\x00"',
            'one="\\x01"',
            'minus_one="\\xff"',
            'sign="\\x80"',
            'limb="\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff"',
            'power_boundary="\\x01\\x00\\x00\\x00\\x00\\x00\\x00\\x00"',
        ]) + "\n"
    )
    (dictionaries / "sequence.dict").write_text(
        '\n'.join([f'op{op}="\\x{op:02x}"' for op in range(11)]) + "\n"
    )


if __name__ == "__main__":
    main()
