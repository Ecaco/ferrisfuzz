use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ferrisfuzz_core::damerau_bp::damerau_bp;      
use ferrisfuzz_core::damerau::damerau;            
use rapidfuzz::distance::osa as rf_osa;          

fn pair(n: usize) -> (String, String) {
    let base: String = "the quick brown fox jumps over the lazy dog "
        .chars().cycle().take(n).collect();
    let mut ed: Vec<char> = base.chars().collect();
    for i in (5..ed.len().saturating_sub(1)).step_by(17) {
        ed.swap(i, i + 1);
    }
    (base, ed.into_iter().collect())
}

fn bench_damerau(c: &mut Criterion) {
    for &n in &[16usize, 48, 64, 128, 256] {
        let (a, b) = pair(n);
        let mut g = c.benchmark_group(format!("osa/{n}"));

        g.bench_function(BenchmarkId::new("ferrisfuzz", "bp"), |bn| {
            bn.iter(|| damerau_bp(black_box(&a), black_box(&b), None, None, None).unwrap())
        });

        g.bench_function(BenchmarkId::new("ferrisfuzz", "classic"), |bn| {
            bn.iter(|| damerau(black_box(&a), black_box(&b), None, None).unwrap())
        });

        // rapidfuzz OSA over chars() — the fair, char-correct comparison.
        g.bench_function(BenchmarkId::new("rapidfuzz", "osa-chars"), |bn| {
            bn.iter(|| rf_osa::distance(black_box(&a).chars(), black_box(&b).chars()))
        });

        g.finish();
    }
}

criterion_group!(benches, bench_damerau);
criterion_main!(benches);