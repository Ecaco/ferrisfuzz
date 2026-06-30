use criterion::{criterion_group, criterion_main, Criterion};
use ferrisfuzz_core::levenshtein::levenshtein_distance;
use ferrisfuzz_core::myers::myers_distance;
use ferrisfuzz_core::jaro_winkler::jaro_winkler;
use ferrisfuzz_core::damerau::damerau;

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

fn bench_jaro(c: &mut Criterion) {
    c.bench_function("jaro kitten>sitting", |b| {
        b.iter(|| jaro_winkler("kitten", "sitting", None, Some(false)))
    });
}

fn bench_damerau(c: &mut Criterion) {
    c.bench_function("damerau kitten>sitting", |b| {
        b.iter(|| damerau("kitten", "sitting"));
    }
);
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

fn bench_jaro_winkler_long(c: &mut Criterion) {
    let s1 = "the quick brown fox jumps over the lazy dog";
    let s2 = "the slow green fox jumped over the lazy cat";
    c.bench_function("jaro winkler long strings", |b| {
        b.iter(|| jaro_winkler(s1, s2, Some(0.0), None) )
    });
}

fn bench_damerau_long(c: &mut Criterion) {
    let s1 = "the quick brown fox jumps over the lazy dog";
    let s2 = "the slow green fox jumped over the lazy cat";
    c.bench_function("damerau long strings", |b| {
        b.iter(|| damerau(s1, s2) )
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

use rapidfuzz::distance::{jaro, levenshtein};

fn bench_rapidfuzz_levenshtein(c: &mut Criterion) {
    c.bench_function("rapidfuzz levenshtein long", |b| {
        b.iter(|| levenshtein::distance(
            "the quick brown fox jumps over the lazy dog".chars(),
            "the slow green fox jumped over the lazy cat".chars(),
        ))
    });
}


criterion_group!(benches, bench_levenshtein, bench_myers, bench_jaro, bench_damerau, bench_levenshtein_long, bench_myers_long, bench_jaro_winkler_long, bench_damerau_long, bench_sweep, bench_rapidfuzz_levenshtein);
criterion_main!(benches);