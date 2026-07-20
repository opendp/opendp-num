use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use opendp_num::{Add, DirectedPowI, Direction, ExactBinary, Mul};

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("exact_integer_256bit");

    #[cfg(feature = "dashu")]
    {
        use dashu::integer::IBig;
        use opendp_num::backend::dashu::Dashu;
        let lhs = (IBig::from(1u8) << 255) - 19u8;
        let rhs = (IBig::from(1u8) << 127) + 7u8;
        group.bench_function("dashu/add", |b| {
            b.iter(|| <Dashu as ExactBinary<Add, IBig>>::eval(black_box(&lhs), black_box(&rhs)))
        });
        group.bench_function("dashu/mul", |b| {
            b.iter(|| <Dashu as ExactBinary<Mul, IBig>>::eval(black_box(&lhs), black_box(&rhs)))
        });
    }
    #[cfg(feature = "malachite")]
    {
        use malachite::Integer;
        use opendp_num::backend::malachite::Malachite;
        let lhs = (Integer::from(1u8) << 255) - Integer::from(19u8);
        let rhs = (Integer::from(1u8) << 127) + Integer::from(7u8);
        group.bench_function("malachite/add", |b| {
            b.iter(|| {
                <Malachite as ExactBinary<Add, Integer>>::eval(black_box(&lhs), black_box(&rhs))
            })
        });
        group.bench_function("malachite/mul", |b| {
            b.iter(|| {
                <Malachite as ExactBinary<Mul, Integer>>::eval(black_box(&lhs), black_box(&rhs))
            })
        });
    }
    #[cfg(feature = "mpfr")]
    {
        use opendp_num::backend::mpfr::Mpfr;
        use rug::Integer;
        let lhs = (Integer::from(1u8) << 255) - 19u8;
        let rhs = (Integer::from(1u8) << 127) + 7u8;
        group.bench_function("mpfr/add", |b| {
            b.iter(|| <Mpfr as ExactBinary<Add, Integer>>::eval(black_box(&lhs), black_box(&rhs)))
        });
        group.bench_function("mpfr/mul", |b| {
            b.iter(|| <Mpfr as ExactBinary<Mul, Integer>>::eval(black_box(&lhs), black_box(&rhs)))
        });
    }
    group.finish();

    #[cfg(feature = "dashu")]
    {
        use opendp_num::backend::dashu::Dashu;
        use opendp_num::{DirectedUnary, Direction, Ln};
        let mut directed = c.benchmark_group("directed_f64");
        directed.bench_function("dashu/ln/down", |b| {
            b.iter(|| {
                <Dashu as DirectedUnary<Ln, f64>>::eval(black_box(1.23456789), Direction::Down)
            })
        });
        directed.bench_function("dashu/ln/up", |b| {
            b.iter(|| <Dashu as DirectedUnary<Ln, f64>>::eval(black_box(1.23456789), Direction::Up))
        });
        directed.finish();
    }

    #[cfg(feature = "dashu")]
    {
        use dashu::{
            float::{DBig, FBig, round::mode::HalfEven},
            integer::IBig,
        };
        use opendp_num::backend::dashu::Dashu;

        let mut powers = c.benchmark_group("directed_powi_bigint");
        let small = IBig::from(53);
        let enormous = (IBig::from(1u8) << 200) + IBig::ONE;
        powers.bench_function("dashu/small", |b| {
            b.iter(|| {
                <Dashu as DirectedPowI<f64, IBig>>::eval(
                    black_box(1.0001),
                    black_box(&small),
                    Direction::Up,
                )
            })
        });
        powers.bench_function("dashu/enormous-classification", |b| {
            b.iter(|| {
                <Dashu as DirectedPowI<f64, IBig>>::eval(
                    black_box(2.0),
                    black_box(&enormous),
                    Direction::Down,
                )
            })
        });
        powers.finish();

        let mut conversions = c.benchmark_group("raw_dashu_float_conversion");
        for bits in [64usize, 128, 256, 1024, 4096] {
            let binary = FBig::<HalfEven, 2>::from_parts(
                (IBig::from(1u8) << (bits - 1)) + IBig::ONE,
                -(bits as isize),
            );
            conversions.bench_function(format!("fbig-to-f64/{bits}-bit"), |b| {
                b.iter(|| black_box(&binary).to_f64())
            });
        }
        let decimal = DBig::from_parts(IBig::from(1234567890123456789012345678901u128), -13);
        conversions.bench_function("dbig-to-f64/overwide", |b| {
            b.iter(|| black_box(&decimal).to_f64())
        });
        conversions.finish();
    }
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
