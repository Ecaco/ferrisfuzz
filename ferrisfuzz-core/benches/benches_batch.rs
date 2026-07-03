//! Batch benchmark: one query vs many candidates, for each bit-parallel metric.
//!
//! Four questions per metric so no cost hides inside another:
//!   1. compile-once + score-all, ours          (realistic end-to-end call)
//!   2. compile-once + score-all, rapidfuzz      (their BatchComparator)
//!   3. score-all only, ours (query precompiled)  (amortized per-candidate cost)
//!   4. score-all only, rapidfuzz                 (same)
//!
//! Groups are named by metric (batch/lev/N, batch/osa/N) so each metric's four-way
//! comparison reads as its own block. Opponents are matched METRIC-for-METRIC:
//! Levenshtein vs levenshtein::BatchComparator, OSA vs osa::BatchComparator.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};

use ferrisfuzz_core::levenshtein_batch::{levenshtein_batch, LevenshteinBatch};
use ferrisfuzz_core::damerau_batch::{damerau_batch, DamerauBatch};
use rapidfuzz::distance::levenshtein as rf_lev;
use rapidfuzz::distance::osa as rf_osa;
use ferrisfuzz_core::jaro_winkler_batch::{jaro_winkler_batch, JaroWinklerBatch};
use rapidfuzz::distance::jaro_winkler as rf_jw;

/// Synthetic candidate list of `n` short ASCII words near the query.
/// A couple of adjacent swaps are sprinkled in so OSA's transposition path is
/// actually exercised (otherwise OSA just does Levenshtein-equivalent work).
fn make_candidates(n: usize) -> Vec<String> {
    let bases = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written"];
    (0..n)
        .map(|i| {
            let mut w = String::from(bases[i % bases.len()]);
            if i % 3 == 0 { w.push((b'a' + (i % 26) as u8) as char); }
            // occasional transposition so OSA does OSA work
            if i % 5 == 0 && w.len() >= 3 {
                let b = unsafe { w.as_bytes_mut() };
                b.swap(1, 2);
            }
            w
        })
        .collect()
}

const QUERY: &str = "kitten";

fn bench_levenshtein(c: &mut Criterion) {
    for &n in &[100usize, 1_000, 10_000] {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/lev/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| {
                let out = levenshtein_batch(black_box(QUERY), black_box(&cands), None);
                black_box(out);
            })
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let scorer = rf_lev::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<usize> = cands.iter().map(|t| scorer.distance(t.chars())).collect();
                black_box(out);
            })
        });

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

fn bench_osa(c: &mut Criterion) {
    for &n in &[100usize, 1_000, 10_000] {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/osa/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| {
                let out = damerau_batch(black_box(QUERY), black_box(&cands), None);
                black_box(out);
            })
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let scorer = rf_osa::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<usize> = cands.iter().map(|t| scorer.distance(t.chars())).collect();
                black_box(out);
            })
        });

        let scorer_ours = DamerauBatch::new(QUERY, None);
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let out: Vec<usize> =
                    cands.iter().map(|t| scorer_ours.distance(black_box(t))).collect();
                black_box(out);
            })
        });
        let scorer_rf = rf_osa::BatchComparator::new(QUERY.chars());
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


fn bench_jaro_winkler(c: &mut Criterion) {
    for &n in &[100usize, 1_000, 10_000] {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/jw/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| {
                let out = jaro_winkler_batch(black_box(QUERY), black_box(&cands), None, None).unwrap();
                black_box(out);
            })
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let scorer = rf_jw::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<f64> = cands.iter().map(|t| scorer.similarity(t.chars())).collect();
                black_box(out);
            })
        });

        let scorer_ours = JaroWinklerBatch::new(QUERY, None, None).unwrap();
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let out: Vec<f64> =
                    cands.iter().map(|t| scorer_ours.similarity(black_box(t))).collect();
                black_box(out);
            })
        });
        let scorer_rf = rf_jw::BatchComparator::new(QUERY.chars());
        g.bench_function(BenchmarkId::new("rapidfuzz", "score-only"), |b| {
            b.iter(|| {
                let out: Vec<f64> =
                    cands.iter().map(|t| scorer_rf.similarity(black_box(t).chars())).collect();
                black_box(out);
            })
        });

        g.finish();
    }
}

criterion_group!(benches_batch, bench_jaro_winkler);
criterion_main!(benches_batch);