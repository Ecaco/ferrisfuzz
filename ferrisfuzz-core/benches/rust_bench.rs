
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};

// ---------------------------------------------------------------------------
// IMPORT BLOCK — the only place name-guesses live.
// ---------------------------------------------------------------------------
use ferrisfuzz_core::levenshtein::levenshtein_distance_classic;
use ferrisfuzz_core::levenshtein_bp::levenshtein_bp;
use ferrisfuzz_core::damerau::damerau_classic;
use ferrisfuzz_core::damerau_bp::damerau_bp;
use ferrisfuzz_core::jaro_winkler::jaro_winkler;
use ferrisfuzz_core::jaro_winkler_classic::jaro_winkler_classic;
use ferrisfuzz_core::levenshtein_batch::{levenshtein_batch, LevenshteinBatch};
use ferrisfuzz_core::damerau_batch::{damerau_batch, DamerauBatch};
use ferrisfuzz_core::jaro_winkler_batch::{jaro_winkler_batch, JaroWinklerBatch};

use rapidfuzz::distance::jaro_winkler as rf_jw;
use rapidfuzz::distance::levenshtein as rf_lev;
use rapidfuzz::distance::osa as rf_osa;

// ---------------------------------------------------------------------------
// Fixed inputs. Three ASCII length classes:
//   SHORT  — 6/7 chars: table-setup dominates; small-string territory.
//   MEDIUM — 15 chars:  sits ON the <=16 JW dispatch boundary, deliberately.
//   LONG   — 44 chars:  wide-window bp, still one u64 word.
// ---------------------------------------------------------------------------
const SHORT: (&str, &str) = ("kitten", "sitting");
const MEDIUM: (&str, &str) = ("acknowledgement", "acknowledgments");
const LONG: (&str, &str) = (
    "the quick brown fox jumps over the lazy dog",
    "the slow green fox jumped over the lazy cat",
);
const PAIRS: [(&str, (&str, &str)); 3] =
    [("short", SHORT), ("medium", MEDIUM), ("long", LONG)];

const QUERY: &str = "kitten";
const BATCH_SIZES: [usize; 3] = [100, 1_000, 10_000];

/// Near-query ASCII words with sprinkled adjacent swaps (exercises OSA's
/// transposition term and Jaro's transposition walk). Deterministic.
fn make_candidates(n: usize) -> Vec<String> {
    let bases = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written"];
    (0..n)
        .map(|i| {
            let mut w = String::from(bases[i % bases.len()]);
            if i % 3 == 0 {
                w.push((b'a' + (i % 26) as u8) as char);
            }
            if i % 5 == 0 && w.len() >= 3 {
                let b = unsafe { w.as_bytes_mut() };
                b.swap(1, 2);
            }
            w
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Single-pair
// ---------------------------------------------------------------------------

fn bench_single_levenshtein(c: &mut Criterion) {
    let mut g = c.benchmark_group("single/lev");
    for (label, (a, b)) in PAIRS {
        g.bench_with_input(BenchmarkId::new("ours-bp", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| levenshtein_bp(black_box(a), black_box(b), None, None, None).unwrap())
        });
        g.bench_with_input(BenchmarkId::new("ours-classic", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| {
                levenshtein_distance_classic(black_box(a), black_box(b), None, None).unwrap()
            })
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-chars", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_lev::distance(black_box(a).chars(), black_box(b).chars()))
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-bytes", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_lev::distance(black_box(a).bytes(), black_box(b).bytes()))
        });
    }
    g.finish();
}

fn bench_single_osa(c: &mut Criterion) {
    let mut g = c.benchmark_group("single/osa");
    for (label, (a, b)) in PAIRS {
        g.bench_with_input(BenchmarkId::new("ours-bp", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| damerau_bp(black_box(a), black_box(b), None, None, None).unwrap())
        });
        g.bench_with_input(BenchmarkId::new("ours-classic", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| damerau_classic(black_box(a), black_box(b), None, None).unwrap())
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-chars", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_osa::distance(black_box(a).chars(), black_box(b).chars()))
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-bytes", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_osa::distance(black_box(a).bytes(), black_box(b).bytes()))
        });
    }
    g.finish();
}

fn bench_single_jaro_winkler(c: &mut Criterion) {
    let mut g = c.benchmark_group("single/jw");
    for (label, (a, b)) in PAIRS {
        g.bench_with_input(BenchmarkId::new("ours", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| {
                jaro_winkler(black_box(a), black_box(b), None, None, None, None).unwrap()
            })
        });
        g.bench_with_input(BenchmarkId::new("ours-classic", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| jaro_winkler_classic(black_box(a), black_box(b), Some(0.1), None, None))
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-chars", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_jw::similarity(black_box(a).chars(), black_box(b).chars()))
        });
        g.bench_with_input(BenchmarkId::new("rapidfuzz-bytes", label), &(a, b), |ben, &(a, b)| {
            ben.iter(|| rf_jw::similarity(black_box(a).bytes(), black_box(b).bytes()))
        });
    }
    g.finish();
}

// ---------------------------------------------------------------------------
// Batch. Barrier discipline: inputs black_boxed ONCE entering the closure,
// output ONCE leaving it. The candidate loop itself is barrier-free — that's
// the code shape a real caller has, for us AND for rapidfuzz.
// ---------------------------------------------------------------------------

fn bench_batch_levenshtein(c: &mut Criterion) {
    for n in BATCH_SIZES {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/lev/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| black_box(levenshtein_batch(black_box(QUERY), black_box(&cands), None)))
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let s = rf_lev::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<usize> = cands.iter().map(|t| s.distance(t.chars())).collect();
                black_box(out)
            })
        });

        let ours = LevenshteinBatch::new(QUERY, None);
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let cands = black_box(&cands); // barrier once — NOT per element
                let out: Vec<usize> = cands.iter().map(|t| ours.distance(t)).collect();
                black_box(out)
            })
        });
        let rf = rf_lev::BatchComparator::new(QUERY.chars());
        g.bench_function(BenchmarkId::new("rapidfuzz", "score-only"), |b| {
            b.iter(|| {
                let cands = black_box(&cands); // same discipline for the opponent
                let out: Vec<usize> = cands.iter().map(|t| rf.distance(t.chars())).collect();
                black_box(out)
            })
        });

        // PROBE (10k only): scorer OWNED by the timed closure, construction
        // untimed. Distinguishes ownership effects from cross-crate call cost.
        // Remove once the score-only vs compile+score gap is explained.
        if n == 10_000 {
            g.bench_function(BenchmarkId::new("ours", "score-only-owned"), |b| {
                b.iter_batched(
                    || LevenshteinBatch::new(QUERY, None),
                    |scorer| {
                        let cands = black_box(&cands);
                        let out: Vec<usize> =
                            cands.iter().map(|t| scorer.distance(t)).collect();
                        black_box(out)
                    },
                    BatchSize::SmallInput,
                )
            });
        }

        g.finish();
    }
}

fn bench_batch_osa(c: &mut Criterion) {
    for n in BATCH_SIZES {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/osa/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| black_box(damerau_batch(black_box(QUERY), black_box(&cands), None)))
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let s = rf_osa::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<usize> = cands.iter().map(|t| s.distance(t.chars())).collect();
                black_box(out)
            })
        });

        let ours = DamerauBatch::new(QUERY, None);
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let out: Vec<usize> = cands.iter().map(|t| ours.distance(t)).collect();
                black_box(out)
            })
        });
        let rf = rf_osa::BatchComparator::new(QUERY.chars());
        g.bench_function(BenchmarkId::new("rapidfuzz", "score-only"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let out: Vec<usize> = cands.iter().map(|t| rf.distance(t.chars())).collect();
                black_box(out)
            })
        });

        g.finish();
    }
}

fn bench_batch_jaro_winkler(c: &mut Criterion) {
    for n in BATCH_SIZES {
        let owned = make_candidates(n);
        let cands: Vec<&str> = owned.iter().map(String::as_str).collect();

        let mut g = c.benchmark_group(format!("batch/jw/{n}"));
        g.throughput(Throughput::Elements(n as u64));

        g.bench_function(BenchmarkId::new("ours", "compile+score"), |b| {
            b.iter(|| {
                black_box(
                    jaro_winkler_batch(black_box(QUERY), black_box(&cands), None, None).unwrap(),
                )
            })
        });
        g.bench_function(BenchmarkId::new("rapidfuzz", "compile+score"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let s = rf_jw::BatchComparator::new(black_box(QUERY).chars());
                let out: Vec<f64> = cands.iter().map(|t| s.similarity(t.chars())).collect();
                black_box(out)
            })
        });

        let ours = JaroWinklerBatch::new(QUERY, None, None).unwrap();
        g.bench_function(BenchmarkId::new("ours", "score-only"), |b| {
            b.iter(|| {
                let cands = black_box(&cands);
                let out: Vec<f64> = cands.iter().map(|t| ours.similarity(t)).collect();
                black_box(out)
            })
        });

        g.finish();
    }
}

fn configured() -> Criterion {
    Criterion::default()
        .warm_up_time(std::time::Duration::from_secs(2))
        .measurement_time(std::time::Duration::from_secs(5))
        .sample_size(100)
}

criterion_group! {
    name = benches;
    config = configured();
    targets =
        bench_single_levenshtein,
        bench_single_osa,
        bench_single_jaro_winkler,
        bench_batch_levenshtein,
        bench_batch_osa,
        bench_batch_jaro_winkler,
}
criterion_main!(benches);