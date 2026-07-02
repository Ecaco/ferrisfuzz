//! Batch benchmark: one query vs many candidates.
//!
//! Four separate questions so no cost hides inside another:
//!   1. compile-once + score-all, ours          (realistic end-to-end call)
//!   2. compile-once + score-all, rapidfuzz      (their BatchComparator)
//!   3. score-all only, ours (query precompiled)  (amortized per-candidate cost)
//!   4. score-all only, rapidfuzz                 (same)

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};

use ferrisfuzz_core::levenshtein_batch::{levenshtein_batch, LevenshteinBatch};
use rapidfuzz::distance::levenshtein as rf_lev;

/// Synthetic candidate list of `n` short ASCII words near the query.
fn make_candidates(n: usize) -> Vec<String> {
    let bases = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written"];
    (0..n)
        .map(|i| {
            let mut w = String::from(bases[i % bases.len()]);
            if i % 3 == 0 { w.push((b'a' + (i % 26) as u8) as char); }
            w
        })
        .collect()
}

const QUERY: &str = "kitten";

fn bench_batch(c: &mut Criterion) {
    for &n in &[100usize, 1_000, 10_000] {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        // 1 & 2: full compile-once + score-all (end to end)
        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| {
                let out = levenshtein_batch(black_box(QUERY), black_box(&cands), None);
                black_box(out);
            })
        });

        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let scorer = rf_lev::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<usize> =
                    cands.iter().map(|t| scorer.distance(t.chars())).collect();
                black_box(out);
            })
        });

        // 3 & 4: score-all only (compile OUTSIDE b.iter → times per-candidate only)
        let scorer_ours = LevenshteinBatch::new(QUERY, None);
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let out: Vec<usize> =
                    cands.iter().map(|t| scorer_ours.distance(black_box(t))).collect();
                black_box(out);
            })
        });

        let scorer_rf = rf_lev::BatchComparator::new(QUERY.chars());
        g.bench_function(BenchmarkId::new("rapidfuzz", "score-only"), |b| {
            b.iter(|| {
                let out: Vec<usize> =
                    cands.iter().map(|t| scorer_rf.distance(black_box(t).chars())).collect();
                black_box(out);
            })
        });

        g.finish();
    }
}

criterion_group!(benches, bench_batch);
criterion_main!(benches);