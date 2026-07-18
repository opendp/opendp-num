use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opendp_num::{Add, ExactBinary, Mul};

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
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
