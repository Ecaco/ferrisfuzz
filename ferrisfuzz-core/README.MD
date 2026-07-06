# ferrisfuzz-core

Fast string similarity metrics — Levenshtein, Damerau (OSA), and
Jaro-Winkler — in pure `no_std` Rust with zero dependencies. Bit-parallel
single-pair and compile-once batch APIs, faster than rapidfuzz-rs (the
fastest existing Rust implementation) in our benchmarks, and embeddable
where a C++-cored library can't go: bare-metal, WASM, and any `no_std`
Rust project (allocator required).

## Benchmarks

Single query vs. rapidfuzz-rs (chars API), criterion min estimates, ns:

| metric       | short (6ch) |       | medium (15ch) |       | long (44ch) |       |
|--------------|------------:|------:|--------------:|------:|------------:|------:|
|              |    ours     |  rf   |     ours      |  rf   |    ours     |  rf   |
| levenshtein  |    30.2     | 102.6 |     56.9      | 76.2  |    144.4    | 194.4 |
| osa          |    32.9     | 56.6  |     61.4      | 74.3  |    156.1    | 207.6 |
| jaro-winkler |    27.9     | 78.2  |     60.3      | 108.9 |    114.9    | 255.8 |

Batch (one query × 10,000 candidates, score-only, Melem/s): levenshtein
**68.4** vs 48.3 · osa **57.1** vs 51.2 · jaro-winkler **52.4** vs 31.2.

rapidfuzz's byte API wins at some short/medium lengths if you can forfeit
Unicode handling — full tables, losses included, in
[BENCHMARKS.md](https://github.com/Ecaco/ferrisfuzz/blob/main/BENCHMARKS.md).

Jaro-Winkler semantics match rapidfuzz exactly (Winkler 0.7 boost
threshold, floored transpositions) — enforced by a hard parity gate on
every test run.

*Intel Core Ultra 7 (P-core pinned), Windows 11, rustc 1.94.1,
rapidfuzz-rs 0.5.0, verified across two baselined runs. Reproduce:
`cargo bench --bench rust_bench`.*

## Install & use

```rust
use ferrisfuzz_core::levenshtein_bp::levenshtein_bp;
use ferrisfuzz_core::levenshtein_batch::LevenshteinBatch;

let d = levenshtein_bp("kitten", "sitting", None, None, None)?;
assert_eq!(d, 3);

// compile the query once, score many candidates
let scorer = LevenshteinBatch::new("kitten", None);
let scores: Vec<usize> = candidates.iter().map(|c| scorer.distance(c)).collect();
```

## API

| metric | single-pair | batch (compile-once) | returns |
|---|---|---|---|
| Levenshtein | `levenshtein_bp` | `LevenshteinBatch` / `levenshtein_batch` | `usize` distance |
| Damerau (OSA) | `damerau_bp` | `DamerauBatch` / `damerau_batch` | `usize` distance |
| Jaro-Winkler | `jaro_winkler` | `JaroWinklerBatch` / `jaro_winkler_batch` | `f64` similarity ∈ [0, 1] |

The `_bp` suffix marks bit-parallel implementations; Jaro-Winkler uses a
different fast path and carries no suffix. All functions return
`Result<_, MatchError>`.

Optional parameters (single-pair):

- `max_len: Option<usize>` — reject inputs above this length with
  `MatchError::InputTooLong`. If your inputs are user-controlled, set
  this: it bounds worst-case work on long strings.
- `score_cutoff` — collapse results you don't care about instead of
  reporting them precisely. For Levenshtein/OSA it is a distance
  **ceiling** (`usize`): distances above it return `cutoff + 1`. For
  Jaro-Winkler it is a similarity **floor** (`f64`): similarities below
  it return `0.0`. In both cases the sentinel lands on the far side of
  your threshold, so `result <= cutoff` / `result >= cutoff` filters
  correctly with no special-casing. The cutoff currently filters
  output; kernel-level early termination is on the roadmap.
- `case_insensitive: Option<bool>` — fold case before comparison.
  Defaults to `false`: comparisons are **case-sensitive** unless you
  opt in.
- Jaro-Winkler only: `p: Option<f64>` — the Winkler prefix weight,
  default 0.1, valid range [0.0, 0.25]; values outside it return
  `MatchError::InvalidPrefixScale`.

## no_std & embedded

`ferrisfuzz-core` is `no_std` with `alloc`: no OS, no threads — an
allocator is required. Single-pair ASCII inputs ≤64 characters run
entirely on stack buffers (no heap allocation) on the default
case-sensitive path; `case_insensitive` lowercases into an owned
`String` first. Longer inputs, non-ASCII inputs, and the batch APIs
allocate.

## Roadmap

- [ ] **Chunked batch ingestion** — batches beyond ~10k candidates
  currently pay a cache penalty (~30% throughput at 50k); chunking
  streams the work.
- [ ] **Kernel-level `score_cutoff` termination** — abort inside the
  scan once the cutoff is unreachable, rather than filtering finished
  results.
- [ ] **SIMD verification stage** — vectorize the inner loops for
  medium/long inputs.
- [ ] **Q-gram prefiltering** — a signature filter stage ahead of
  scoring, so large candidate pools are pruned before the metric runs.

## License

Licensed under either of Apache-2.0 or MIT, at your option.