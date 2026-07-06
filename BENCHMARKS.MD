# Benchmarks

Full results for `ferrisfuzz-core` vs [rapidfuzz-rs], measured with criterion.
Every number below comes from a single pinned, baselined pair of runs on one
binary — methodology at the bottom. Cells we lose are included and marked.

**Conditions:** Intel Core Ultra 7 (pinned to one P-core), Windows 11,
rustc 1.94.1, rapidfuzz-rs 0.5, criterion min estimates, 100 samples / 5s
measurement time, ASCII inputs. Verified across two baselined runs
(`--save-baseline` / `--baseline`); all cells reproduced within ±3% unless
footnoted. Reproduce: `cargo bench --bench rust_bench`.

## Single-pair

Inputs: short = `kitten`/`sitting` (6/7 ch), medium =
`acknowledgement`/`acknowledgments` (15 ch), long = two 43-char sentences.
All times ns, criterion min estimates.

| bench | ours | ours (classic) | rapidfuzz (chars) | rapidfuzz (bytes) |
|---|---:|---:|---:|---:|
| lev / short   | **30.2** | 281.9 | 102.6 | 39.3 |
| lev / medium  | 56.9 | 663–1008 ¹ | 76.2 | **34.6** ² |
| lev / long    | **144.4** | 3 625 | 194.4 | 204.5 |
| osa / short   | 32.9 | 328.0 | 56.6 | **28.4** ² |
| osa / medium  | 61.4 | 780.4 | 74.3 | **34.7** ² |
| osa / long    | **156.1** | 4 331 | 207.6 | 209.5 |
| jw / short    | **27.9** | 111.1 | 78.2 | 61.4 |
| jw / medium   | **60.3** | 313.9 | 108.9 | 63.8 |
| jw / long     | **114.9** | 255.6 | 255.8 | 249.5 |

**Bold** = fastest in row.

The `chars` column is the primary comparison: it is rapidfuzz's
Unicode-correct API, matching what our public functions do internally
(we accept `&str`, detect ASCII, and dispatch to a byte path ourselves).
The `bytes` column is rapidfuzz's raw-byte API, which requires the caller
to forfeit Unicode handling; it is included for completeness and wins
three cells at short/medium lengths (²). Its advantage inverts at long
lengths on every metric.

¹ The classic (reference) implementations are naive O(mn) baselines, not
shipped code paths. The lev/medium classic cell is bimodal across
*processes* (≈663ns or ≈1 008ns, same binary, same pin), consistent with
allocation-alignment sensitivity in the naive `Vec<bool>` loops. The
optimized paths show no such behavior.

## Batch (one query × N candidates)

`compile+score` constructs the scorer inside the timed region (one-shot
cost); `score-only` reuses a pre-built scorer (hot-loop cost). Throughput
in Melem/s, higher is better.

| bench | ours c+s | ours s-o | rapidfuzz c+s | rapidfuzz s-o |
|---|---:|---:|---:|---:|
| lev / 100    | **63.3** | 61.7 | 45.4 | 47.7 |
| lev / 1 000  | **68.5** | 67.5 | 47.4 | 48.4 |
| lev / 10 000 | **68.6** | 68.4 | 47.6 | 48.3 |
| osa / 100    | **54.8** | 53.9 | 49.5 | 51.6 |
| osa / 1 000  | **57.5** | 57.0 | 50.9 | 52.2 |
| osa / 10 000 | **57.4** | 57.1 | 50.6 | 51.2 |
| jw / 100     | 48.0 | **50.5** | 29.5 | 31.4 ³ |
| jw / 1 000   | 51.9 | **52.6** | 31.1 | 31.4 ³ |
| jw / 10 000  | 52.1 | **52.4** | 30.9 | 31.2 |

For all three metrics, `compile+score ≈ score-only` on our side:
scorer construction (including the 1 KiB pattern table) amortizes to
zero even at N=100, so there is no penalty for one-shot batch calls.

³ rapidfuzz jw score-only at 100/1 000 showed a single-cell +9–12%
regression in the verification run (transient interference); the
values shown are from the recorded baseline run, consistent with all
prior measurements of those cells.

## Python (the wheel, as installed)

Measured with pytest-benchmark (min times, GC disabled, 1 000
iterations per measurement for single-pair). These numbers include the
Python↔Rust FFI boundary; correctness gates (three-way parity vs an
independent oracle and vs rapidfuzz) run in the same session that
produces the table.

Single-pair, ns, vs the rapidfuzz Python package:

| metric | short | | medium | | long | |
|---|---:|---:|---:|---:|---:|---:|
| | ours | rf | ours | rf | ours | rf |
| levenshtein | **82.5** | 160.7 | **107.7** | 128.7 | **196.3** | 236.6 |
| osa | **82.7** | 141.7 | **107.4** | 134.1 | **192.8** | 245.9 |
| jaro-winkler | **84.6** | 235.6 | **121.5** | 258.5 | **174.4** | 400.9 |

Subtracting the Rust-core times above gives a measured FFI cost of
~50–60ns per call, flat across input lengths and metrics.

Batch, one query × N, total µs (min), vs `rapidfuzz.process.cdist`:

| metric | 1 000 | | 5 000 | | 50 000 | |
|---|---:|---:|---:|---:|---:|---:|
| | ours | cdist | ours | cdist | ours | cdist |
| levenshtein | **27.5** | 50.2 | **137.0** | 240.0 | **1 815** | 2 501 |
| osa | **29.4** | 35.4 | **147.4** | 169.6 | 2 403 ⁴ | **2 025** |
| jaro-winkler | **36.4** | 82.0 | **182.1** | 407.4 | **2 767** | 4 273 |

⁴ **A cell we lose.** At 50 000 candidates our per-element cost rises
~30–50% across all metrics while rapidfuzz's rises ~10–20%; for OSA
this flips the comparison. Boundary probes in `python_bench.py`
(`test_probe_ingest`) isolate the mechanism: the working set exceeds
cache and our multi-pass ingestion (convert list → score) pays the
memory latency more than once, where a streaming design pays it once.
Chunked ingestion is the planned fix — tracked in [issue #N].

Batches of ≥10 000 candidates release the GIL during computation
(verified by `test_detach_composes_with_threads`, which asserts a
concurrent Python thread makes >1.5× progress vs the GIL-held case).

## Methodology

- **Pinning:** benches run via `start /affinity 0x1` on a single P-core;
  Intel hybrid scheduling otherwise moves work between P- and E-cores
  mid-run, producing 30–50% phantom swings.
- **Pairing:** every published Rust number comes from a
  `--save-baseline` run verified by a subsequent `--baseline` run on
  the same binary after a cool-down; cells that fail to reproduce are
  investigated or footnoted, never silently kept.
- **Barriers:** `black_box` is applied once at the boundary of each
  timed region, never per-element inside candidate loops — per-element
  barriers measurably block the inlining a real caller's loop receives
  (we measured ~0.8ns/call of instrumentation overhead before fixing
  this).
- **Fairness:** rapidfuzz OSA is compared against rapidfuzz's `osa`
  module (never `damerau_levenshtein`, a different metric); Jaro-Winkler
  compares `similarity` on both sides; our JW semantics match
  rapidfuzz's exactly (Winkler 0.7 boost threshold, floored
  transpositions), enforced by a hard parity gate on every test run.
- **Correctness before speed:** all optimized paths are cross-checked
  against independent naive oracles on 20 000+ adversarial pairs per
  metric; the Python table is produced in the same pytest session as
  the parity gates, so a timing table cannot be generated from a wrong
  build.