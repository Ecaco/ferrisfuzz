use criterion::{criterion_group, criterion_main, Criterion};
use ferrisfuzz_core::levenshtein::levenshtein_distance;
use ferrisfuzz_core::myers::myers_distance;

fn bench_levenshtein(c: &mut Criterion) {
    c.bench_function("levenshtein kitten→sitting", |b| {
        b.iter(|| levenshtein_distance("kitten", "sitting"))
    });
}

fn bench_myers(c: &mut Criterion) {
    c.bench_function("myers kitten→sitting", |b| {
        b.iter(|| myers_distance("kitten", "sitting", None))
    });
}

fn bench_levenshtein_long(c: &mut Criterion) {
    let s1 = "the quick brown fox jumps over the lazy dog";
    let s2 = "the slow green fox jumped over the lazy cat";
    c.bench_function("levenshtein long strings", |b| {
        b.iter(|| levenshtein_distance(s1, s2))
    });
}

fn bench_myers_long(c: &mut Criterion) {
    let s1 = "the quick brown fox jumps over the lazy dog";
    let s2 = "the slow green fox jumped over the lazy cat";
    c.bench_function("myers long strings", |b| {
        b.iter(|| myers_distance(s1, s2, None))
    });
}

fn bench_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("length sweep");
    
    let pairs = [
        (4,  "rust", "bust"),
        (8,  "rustlang", "bullying"),
        (16, "the quick brown!", "the slow  green!"),
        (32, "the quick brown fox jumps over!", "the slow green fox jumped over!"),
        (43, "the quick brown fox jumps over the lazy dog", "the slow green fox jumped over the lazy cat"),
    ];
    
    for (len, s1, s2) in pairs {
        group.bench_with_input(
            format!("myers_{}", len),
            &(s1, s2),
            |b, (s1, s2)| b.iter(|| myers_distance(s1, s2, None))
        );
    }
    
    group.finish();
}

use rapidfuzz::distance::levenshtein;

fn bench_rapidfuzz_levenshtein(c: &mut Criterion) {
    c.bench_function("rapidfuzz levenshtein long", |b| {
        b.iter(|| levenshtein::distance(
            "the quick brown fox jumps over the lazy dog".chars(),
            "the slow green fox jumped over the lazy cat".chars(),
        ))
    });
}
//criterion_group!(benches, bench_levenshtein, bench_myers, bench_levenshtein_long, bench_myers_long, bench_sweep);
criterion_group!(benches, bench_rapidfuzz_levenshtein);
criterion_main!(benches);