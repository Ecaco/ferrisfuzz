# ferrisfuzz

[![CI](https://github.com/Ecaco/ferrisfuzz/actions/workflows/CI.yml/badge.svg)](https://github.com/Ecaco/ferrisfuzz/actions/workflows/CI.yml)
[![crates.io](https://img.shields.io/crates/v/ferrisfuzz-core.svg)](https://crates.io/crates/ferrisfuzz-core)
[![docs.rs](https://img.shields.io/docsrs/ferrisfuzz-core)](https://docs.rs/ferrisfuzz-core)
[![PyPI](https://img.shields.io/pypi/v/ferrisfuzz.svg)](https://pypi.org/project/ferrisfuzz/)

Python bindings for [`ferrisfuzz-core`](https://github.com/Ecaco/ferrisfuzz) —
fast string similarity metrics (Levenshtein, Damerau/OSA, and Jaro-Winkler)
backed by a pure `no_std` Rust core with zero dependencies. Bit-parallel
single-pair scoring and GIL-releasing batch APIs, faster than the rapidfuzz
Python package across most inputs in our benchmarks, with the Python↔Rust
boundary measured.


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

## License

Licensed under either of Apache-2.0 or MIT, at your option.