# ferrisfuzz

[![CI](https://github.com/YOURUSER/ferrisfuzz/actions/workflows/ci.yml/badge.svg)](https://github.com/Ecaco/ferrisfuzz/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ferrisfuzz-core.svg)](https://crates.io/crates/ferrisfuzz-core)
[![PyPI](https://img.shields.io/pypi/v/ferrisfuzz.svg)](https://pypi.org/project/ferrisfuzz/)
[![docs.rs](https://img.shields.io/docsrs/ferrisfuzz-core)](https://docs.rs/ferrisfuzz-core)

Fast string similarity metrics — Levenshtein, Damerau (OSA), and
Jaro-Winkler — for Rust and Python. Pure `no_std` Rust core with zero
dependencies; bit-parallel single-pair and compile-once batch APIs;
faster than rapidfuzz-rs in our benchmarks, with Jaro-Winkler semantics
locked to rapidfuzz and enforced by a hard parity gate on every commit.

## Installation

```
cargo add ferrisfuzz-core
```
```
pip install ferrisfuzz
```


## Benchmarks

|  | Rust: single-pair, long, ns | Rust: batch 10k, Melem/s | Python: single-pair, long, ns |
|---|---:|---:|---:|
| **levenshtein** | **144.4** vs 194.4 | **68.4** vs 48.3 | **196.3** vs 236.6 |
| **osa** | **156.1** vs 207.6 | **57.1** vs 51.2 | **192.8** vs 245.9 |
| **jaro-winkler** | **114.9** vs 255.8 | **52.4** vs 31.2 | **174.4** vs 400.9 |

*vs rapidfuzz-rs 0.5.0 (Rust) and the rapidfuzz Python package. Criterion /
pytest-benchmark min estimates, Intel Core Ultra 7 (P-core pinned),
verified across two baselined runs. Full tables (including the cells we
lose, with mechanisms) in [BENCHMARKS.md](BENCHMARKS.md). Per-audience
detail: [Rust README](ferrisfuzz-core/README.md) ·
[Python README](py/README.md).*

## How it was built

**Correctness before speed.** The initial implementations were hand-coded,
non-bit-parallel references following the textbook algorithms, with a
three-way parity gate against an independent Python oracle and rapidfuzz.
The bit-parallel implementations were then built against those references,
with the same gates enforcing correctness on every commit — the references
and gates are in the repo, and CI runs them on every build.

**Semantics locked to the reference.** A parity gate caught our
Jaro-Winkler diverging from rapidfuzz on two inputs. The residual on one
was exactly 1/350 — clean enough to solve backwards: it implied a
transposition count floored to whole transpositions at m=35 matches.
rapidfuzz's source confirmed the floor, plus a Winkler boost threshold we
also lacked. Both were adopted, pinned as named regression tests, and
agreement with rapidfuzz is now a hard gate on every commit.

**Benchmarks as evidence.** All benchmarks are reproducible with the
included harnesses, and were pinned to a single P-core to avoid hybrid
scheduling swings, with every published number verified across two
baselined runs. `black_box` is applied once at the boundary of each timed
region, never per-element inside candidate loops — we measured ~0.8ns/call
of our own instrumentation overhead before fixing this. Methodology and
full tables, losses included, in [BENCHMARKS.md](BENCHMARKS.md).

**The FFI boundary, measured.** Subtracting core (criterion) times from
wheel (pytest) times isolates the Python↔Rust boundary cost: ~50–60ns per
call, flat across input lengths and metrics, replicated across three
benchmark campaigns. Batch calls ingest candidate lists zero-copy, and
batches ≥10k candidates release the GIL — verified by a concurrency test
in the suite.

**Known limits, tracked.** Past ~10k candidates, batch throughput dips
~30% as the working set leaves cache; boundary probes in the repo isolate
the mechanism, and chunked ingestion is the designed fix, tracked in the
roadmap.

## Roadmap

Chunked batch ingestion · kernel-level `score_cutoff` termination ·
SIMD verification stage · q-gram prefiltering

## License

MIT OR Apache-2.0, at your option.

## Acknowledgements

**Algorithms:**
- Myers, G. (1999). [*A Fast Bit-Vector Algorithm for Approximate String
  Matching Based on Dynamic Programming*](https://dl.acm.org/doi/10.1145/316542.316550).
  Journal of the ACM 46(3) — the bit-parallel Levenshtein core
  implemented here.
- Hyyrö, H. (2003). *Explaining and Extending the Bit-parallel
  Approximate String Matching Algorithm of Myers* — the standard
  readable treatment of the algorithm above.
- [Coglan's Myers-diff series](https://blog.jcoglan.com/2017/02/12/the-myers-diff-algorithm-part-1/)
  and the [swift-algorithm-club Myers writeup](https://github.com/kodecocodes/swift-algorithm-club/blob/master/Myers%20Difference%20Algorithm/README.md)
  — where I first learned the edit-graph model that underlies these
  metrics. (These cover Myers' 1986 diff algorithm, distinct from the
  1999 bit-vector algorithm implemented in this crate.)
- [GeeksforGeeks: Levenshtein](https://www.geeksforgeeks.org/dsa/introduction-to-levenshtein-distance/),
  [Damerau-Levenshtein](https://www.geeksforgeeks.org/dsa/damerau-levenshtein-distance/),
  [Jaro-Winkler](https://www.geeksforgeeks.org/dsa/jaro-and-jaro-winkler-similarity/),
  and [Rosetta Code: Jaro-Winkler](https://rosettacode.org/wiki/Jaro-Winkler_distance)
  — first-pass understanding of the classic formulations, used to build
  the reference implementations.

**Reference implementation & benchmark opponent:**
- [rapidfuzz-rs](https://github.com/rapidfuzz/rapidfuzz-rs) and
  [RapidFuzz](https://github.com/rapidfuzz/RapidFuzz) — Jaro-Winkler
  semantics (transposition flooring, Winkler boost threshold) verified
  directly against their source; also the benchmark baseline throughout.

**Books:**
- Hyde, *Write Great Code, Volume 2: Thinking Low-Level* — low-level
  performance reasoning.
- Blandy, Orendorff & Tindall, *Programming Rust: Fast, Safe Systems
  Development.*