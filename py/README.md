# ferrisfuzz

[![CI](https://github.com/YOURUSER/ferrisfuzz/actions/workflows/ci.yml/badge.svg)](https://github.com/YOURUSER/ferrisfuzz/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ferrisfuzz-core.svg)](https://crates.io/crates/ferrisfuzz-core)
[![docs.rs](https://img.shields.io/docsrs/ferrisfuzz-core)](https://docs.rs/ferrisfuzz-core)
[![PyPI](https://img.shields.io/pypi/v/ferrisfuzz.svg)](https://pypi.org/project/ferrisfuzz/)

Python bindings for [`ferrisfuzz-core`](https://github.com/Ecaco/ferrisfuzz) —
fast string similarity metrics (Levenshtein, Damerau/OSA, and Jaro-Winkler)
backed by a pure `no_std` Rust core with zero dependencies. Bit-parallel
single-pair scoring and GIL-releasing batch APIs, faster than the rapidfuzz
Python package across most inputs in our benchmarks, with the Python↔Rust
boundary measured.

## Benchmarks

Measured with pytest-benchmark (min times, GC disabled, 1 000 iterations per
measurement for single-pair). These numbers include the Python↔Rust FFI
boundary; three-way correctness gates (an independent Python oracle and
rapidfuzz) run in the same session that produces the table.

Single query vs. the rapidfuzz Python package, ns:

| metric       | short (6ch) |       | medium (15ch) |       | long (44ch) |       |
|--------------|------------:|------:|--------------:|------:|------------:|------:|
|              |    ours     |  rf   |     ours      |  rf   |    ours     |  rf   |
| levenshtein  |  **82.5**   | 160.7 |   **107.7**   | 128.7 |  **196.3**  | 236.6 |
| osa          |  **82.7**   | 141.7 |   **107.4**   | 134.1 |  **192.8**  | 245.9 |
| jaro-winkler |  **84.6**   | 235.6 |   **121.5**   | 258.5 |  **174.4**  | 400.9 |

Subtracting the Rust-core times gives a measured FFI cost of ~50–60ns per
call, flat across input lengths and metrics.

Batch (one query × N candidates), total µs (min), vs `rapidfuzz.process.cdist`:

| metric       | 1 000  |       | 5 000   |       | 50 000  |       |
|--------------|-------:|------:|--------:|------:|--------:|------:|
|              |  ours  | cdist |  ours   | cdist |  ours   | cdist |
| levenshtein  | **27.5** | 50.2 | **137.0** | 240.0 | **1 815** | 2 501 |
| osa          | **29.4** | 35.4 | **147.4** | 169.6 |  2 403  | **2 025** |
| jaro-winkler | **36.4** | 82.0 | **182.1** | 407.4 | **2 767** | 4 273 |

Batches of ≥10 000 candidates release the GIL during computation, so other
Python threads keep running (verified by
`test_detach_composes_with_threads`). The one cell we lose — OSA at 50 000 —
is a cache-pressure effect in multi-pass ingestion; chunked ingestion is the
planned fix. Full tables, methodology, and losses in
[BENCHMARKS.md](https://github.com/Ecaco/ferrisfuzz/blob/main/BENCHMARKS.MD).

Jaro-Winkler semantics match rapidfuzz exactly (Winkler 0.7 boost threshold,
floored transpositions) — enforced by a hard parity gate on every test run.

*Intel Core Ultra 7 (P-core pinned), Windows 11, rapidfuzz (Python package),
min estimates. Reproduce: `pytest python_bench.py --benchmark-disable-gc
--benchmark-sort=name`.*

## Install

```bash
pip install ferrisfuzz
```

Local development build from source (requires a Rust toolchain and
[maturin](https://www.maturin.rs/)):

```bash
pip install maturin
maturin develop --release
```

## Use

```python
import ferrisfuzz

# single pair
d = ferrisfuzz.levenshtein_bp("kitten", "sitting")
assert d == 3

sim = ferrisfuzz.jaro_winkler_similarity("MARTHA", "MARHTA")
assert 0.0 <= sim <= 1.0

# one query scored against many candidates
query = "kitten"
candidates = ["sitting", "mitten", "kitchen", "bitten"]
scores = ferrisfuzz.levenshtein_batch(query, candidates)   # list[int]
```

## API

| metric        | single-pair               | batch                  | returns                     |
|---------------|---------------------------|------------------------|-----------------------------|
| Levenshtein   | `levenshtein_bp`          | `levenshtein_batch`    | `int` distance              |
| Damerau (OSA) | `damerau_bp`              | `damerau_batch`        | `int` distance              |
| Jaro-Winkler  | `jaro_winkler_similarity` | `jaro_winkler_batch`   | `float` similarity ∈ [0, 1] |

Signatures:

```python
levenshtein_bp(str_1, str_2, max_len=None, case_insensitive=None, score_cutoff=None) -> int
damerau_bp(str_1, str_2, max_len=None, case_insensitive=None, score_cutoff=None) -> int
jaro_winkler_similarity(str_1, str_2, p=None, max_len=None, case_insensitive=None, score_cutoff=None) -> float

levenshtein_batch(query, candidates, case_insensitive=None) -> list[int]
damerau_batch(query, candidates, case_insensitive=None) -> list[int]
jaro_winkler_batch(query, candidates, p=None, case_insensitive=None) -> list[float]
```

The `_bp` suffix marks bit-parallel implementations; Jaro-Winkler uses a
different fast path and carries no suffix. Invalid arguments (e.g. an
out-of-range `p`, or input longer than `max_len`) raise `ValueError`.

### Optional parameters

- `max_len` — reject inputs longer than this with `ValueError`. If your inputs
  are user-controlled, set this: it bounds worst-case work on long strings.
  *(Single-pair only.)*
- `score_cutoff` — collapse results you don't care about instead of reporting
  them precisely. For Levenshtein/OSA it is a distance **ceiling**: distances
  above it return `cutoff + 1`. For Jaro-Winkler it is a similarity **floor**:
  similarities below it return `0.0`. In both cases the sentinel lands on the
  far side of your threshold, so `result <= cutoff` / `result >= cutoff`
  filters correctly with no special-casing. *(Single-pair only.)*
- `case_insensitive` — fold case before comparison. Defaults to `False`:
  comparisons are **case-sensitive** unless you opt in.
- `p` *(Jaro-Winkler only)* — the Winkler prefix weight, default `0.1`, valid
  range `[0.0, 0.25]`; values outside it raise `ValueError`.

The batch APIs deliberately omit `max_len` and `score_cutoff` — they score
every candidate against a compiled query.

## Threading & the GIL

Batch calls with **≥ 10 000 candidates** release the GIL during the scoring
loop, so other Python threads run concurrently while Rust works. Smaller
batches hold the GIL (the release/re-acquire overhead isn't worth it at that
size). This is transparent — the API is identical either way.

## Roadmap

- [ ] **Chunked batch ingestion** — batches beyond ~10k candidates pay a cache
  penalty (~30% throughput at 50k); chunking streams the work.
- [ ] **Kernel-level `score_cutoff` termination** — abort inside the scan once
  the cutoff is unreachable, rather than filtering finished results.
- [ ] **SIMD verification stage** — vectorize the inner loops for medium/long
  inputs.
- [ ] **Q-gram prefiltering** — a signature filter stage ahead of scoring, so
  large candidate pools are pruned before the metric runs.

## License

Licensed under either of Apache-2.0 or MIT, at your option.