use criterion::{black_box, criterion_group, criterion_main, Criterion};
// adjust these `use` paths to match your crate layout
use ferrisfuzz_core::levenshtein_bp::levenshtein_bp;
use ferrisfuzz_core::levenshtein::levenshtein_distance;
use ferrisfuzz_core::damerau::damerau;

const SHORT: (&str, &str) = ("kitten", "sitting");
const LONG: (&str, &str) = (
    "the quick brown fox jumps over the lazy dog",
    "the slow green fox jumped over the lazy cat",
);

fn bench_short(c: &mut Criterion) {
    let (a, b) = SHORT;
    let mut g = c.benchmark_group("edit distance - short");
    g.bench_function("classic", |bn| {
        bn.iter(|| levenshtein_distance(black_box(a), black_box(b), None, None))
    });
    g.bench_function("bit-parallel", |bn| {
        bn.iter(|| levenshtein_bp(black_box(a), black_box(b)))
    });
    g.bench_function("damerau", |bn| {
        bn.iter(|| damerau(black_box(a), black_box(b), None, None))
    });
    g.finish();
}

fn bench_long(c: &mut Criterion) {
    let (a, b) = LONG;
    let mut g = c.benchmark_group("edit distance - long");
    g.bench_function("classic", |bn| {
        bn.iter(|| levenshtein_distance(black_box(a), black_box(b), None, None))
    });
    g.bench_function("bit-parallel", |bn| {
        bn.iter(|| levenshtein_bp(black_box(a), black_box(b)))
    });
    g.bench_function("damerau", |bn| {
        bn.iter(|| damerau(black_box(a), black_box(b), None, None))
    });
    g.finish();
}

criterion_group!(benches, bench_short, bench_long);
criterion_main!(benches);