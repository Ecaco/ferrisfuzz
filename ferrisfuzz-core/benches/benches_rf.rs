use criterion::{black_box, criterion_group, criterion_main, Criterion};
// adjust these `use` paths to match your crate layout
use ferrisfuzz_core::levenshtein_bp::levenshtein_bp;
use ferrisfuzz_core::levenshtein::levenshtein_distance;
use ferrisfuzz_core::damerau::damerau;
use rapidfuzz::distance::levenshtein as rf_lev;

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
        // benches:
        bn.iter(|| levenshtein_bp(black_box(a), black_box(b), None, None, None).unwrap())
    });
    g.bench_function("damerau", |bn| {
        bn.iter(|| damerau(black_box(a), black_box(b), None, None))
    });
    // rapidfuzz over chars() — the fair comparison (yours decodes chars too)
    g.bench_function("rapidfuzz (chars)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(a).chars(), black_box(b).chars()))
    });
    // rapidfuzz over bytes() — their ASCII fast-path; shows the ceiling you'd chase with a bytes path
    g.bench_function("rapidfuzz (bytes)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(a).bytes(), black_box(b).bytes()))
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
// benches:
        bn.iter(|| levenshtein_bp(black_box(a), black_box(b), None, None, None).unwrap())
    });
    g.bench_function("damerau", |bn| {
        bn.iter(|| damerau(black_box(a), black_box(b), None, None))
    });
    g.bench_function("rapidfuzz (chars)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(a).chars(), black_box(b).chars()))
    });
    g.bench_function("rapidfuzz (bytes)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(a).bytes(), black_box(b).bytes()))
    });
    g.finish();
}

/// Exercises the MULTIWORD path (query > 64 chars). Two sizes so you can see it scale
/// across word counts: ~80 chars = 2 u64 words, ~150 chars = 3 words.
fn bench_multiword(c: &mut Criterion) {
    // Built at runtime so the compiler can't const-fold them; black_box seals it.
    let base: String = "the quick brown fox jumps over the lazy dog while the cat sleeps"
        .chars().cycle().take(80).collect();
    let edited: String = "the slow green fox jumped over the lazy cat while the dog barks!"
        .chars().cycle().take(80).collect();

    let base_long: String = base.chars().cycle().take(150).collect();
    let edited_long: String = edited.chars().cycle().take(150).collect();

    let mut g = c.benchmark_group("edit distance - multiword");

    // 80 chars (2 words)
    g.bench_function("classic 80", |bn| {
        bn.iter(|| levenshtein_distance(black_box(&*base), black_box(&*edited), None, None))
    });
    g.bench_function("bit-parallel 80", |bn| {
// benches:
        bn.iter(|| levenshtein_bp(black_box(&*base), black_box(&*edited), None, None, None).unwrap())
    });
    g.bench_function("rapidfuzz 80 (chars)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(&*base).chars(), black_box(&*edited).chars()))
    });

    // 150 chars (3 words)
    g.bench_function("classic 150", |bn| {
        bn.iter(|| levenshtein_distance(black_box(&*base_long), black_box(&*edited_long), None, None))
    });
    g.bench_function("bit-parallel 150", |bn| {
// benches:
        bn.iter(|| levenshtein_bp(black_box(&*base_long), black_box(&*edited_long), None, None, None).unwrap())
    });
    g.bench_function("rapidfuzz 150 (chars)", |bn| {
        bn.iter(|| rf_lev::distance(black_box(&*base_long).chars(), black_box(&*edited_long).chars()))
    });

    g.finish();
}

criterion_group!(benches, bench_short, bench_long, bench_multiword);
criterion_main!(benches);