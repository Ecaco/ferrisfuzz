"""
DIFFERENTIAL PROFILE of the batch FFI boundary.

Goal: account for every microsecond between the pure-Rust batch cost (~160us for
10k, measured in criterion) and the Python-observed ~503us. We add ONE stage at a
time; each stage's cost is the difference between consecutive rows.

Some probes need trivial helper #[pyfunction]s in the Rust binding. Where a probe
requires one, it's guarded by hasattr(...) and SKIPS with a printed note if absent,
telling you exactly what to add. Nothing here is fatal if a helper is missing.

Run:  pytest profile_batch_boundary.py --benchmark-min-rounds=50 -q
Read the MIN column.
"""

import numpy as np
import pytest
import ferrisfuzz

N = 10_000
QUERY = "kitten"
TRIVIAL_QUERY = "k"   # 1-char query => minimal compute, same ingestion
_BASES = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written", "smitten", "cat"]
CANDIDATES = [_BASES[i % len(_BASES)] + ("s" if i % 3 == 0 else "") for i in range(N)]


# ---------------------------------------------------------------------------
# STAGE 0 — pure Python baseline: how long merely to TOUCH the list in Python?
# Establishes the floor. If iterating 10k Python strings is already costly,
# some of the "343us" was never Rust's fault.
# ---------------------------------------------------------------------------

def test_00_python_iterate_only(benchmark):
    # Touch every string (force the interpreter to walk the list + strings)
    def touch():
        total = 0
        for s in CANDIDATES:
            total += len(s)
        return total
    benchmark(touch)


# ---------------------------------------------------------------------------
# STAGE 1 — INGESTION with (near) zero compute.
# Same 10k strings cross the boundary, but the query is 1 char => the Myers loop
# does almost nothing. If this is ~same as the full call, INGESTION dominates and
# compute is negligible at this shape. THIS IS THE KEY TEST.
# ---------------------------------------------------------------------------

def test_10_full_call_trivial_query(benchmark):
    benchmark(ferrisfuzz.levenshtein_batch, TRIVIAL_QUERY, CANDIDATES)


# ---------------------------------------------------------------------------
# STAGE 1b — INGESTION with ZERO compute, if the binding exposes a probe.
# Needs a Rust helper, e.g.:
#     #[pyfunction] fn _ingest_only(candidates: Vec<String>) -> usize { candidates.len() }
# Ingests the list into Vec<String> and returns only the count. No scoring at all.
# The difference (this vs test_10) = the cost of the trivial scoring loop.
# The value itself = pure ingestion cost.
# ---------------------------------------------------------------------------

@pytest.mark.skipif(not hasattr(ferrisfuzz, "_ingest_only"),
                    reason="add `_ingest_only(candidates: Vec<String>) -> usize` to measure pure ingestion")
def test_11_ingest_only(benchmark):
    benchmark(ferrisfuzz._ingest_only, CANDIDATES)


# ---------------------------------------------------------------------------
# STAGE 1c — INGESTION as &str (borrow) instead of String (copy), if exposed.
# Needs e.g.:
#     #[pyfunction] fn _ingest_borrow(candidates: Vec<PyBackedStr>) -> usize { candidates.len() }
#   (or Vec<&str>). Tells you how much of ingestion is the STRING COPY vs the
#   unavoidable per-object boundary crossing. If borrow is much cheaper, the copy
#   is the tax and a zero-copy signature is the fix.
# ---------------------------------------------------------------------------

@pytest.mark.skipif(not hasattr(ferrisfuzz, "_ingest_borrow"),
                    reason="add a borrowing `_ingest_borrow` to measure copy vs crossing")
def test_12_ingest_borrow(benchmark):
    benchmark(ferrisfuzz._ingest_borrow, CANDIDATES)


# ---------------------------------------------------------------------------
# STAGE 3 — FULL real call (ingest + compile + score + marshal). The number we
# are trying to explain.
# ---------------------------------------------------------------------------

def test_30_full_call_real_query(benchmark):
    benchmark(ferrisfuzz.levenshtein_batch, QUERY, CANDIDATES)


# ---------------------------------------------------------------------------
# STAGE 4 — pass pre-encoded bytes instead of str, if the binding exposes it.
# Needs e.g. `levenshtein_batch_bytes(query, candidates: Vec<Vec<u8>>)`.
# Python `bytes` -> Rust `Vec<u8>` skips UTF-8 validation that `str`->`String`
# performs. If this is notably faster, UTF-8 VALIDATION during ingestion is part
# of the tax.
# ---------------------------------------------------------------------------

@pytest.mark.skipif(not hasattr(ferrisfuzz, "levenshtein_batch_bytes"),
                    reason="add a bytes-taking batch fn to measure UTF-8 validation cost")
def test_40_full_call_bytes(benchmark):
    cbytes = [c.encode("ascii") for c in CANDIDATES]
    benchmark(ferrisfuzz.levenshtein_batch_bytes, QUERY.encode("ascii"), cbytes)


def test_batch_sum_scalar_return(benchmark):
    benchmark(ferrisfuzz.levenshtein_batch_sum, QUERY, CANDIDATES)

def test_batch_full_list_return(benchmark):
    benchmark(ferrisfuzz.levenshtein_batch, QUERY, CANDIDATES)

from rapidfuzz.distance import Levenshtein as RapidLevenshtein
from rapidfuzz import process as rf_process

def _cdist_numpy(query, candidates):
    # cdist wants a collection of queries; wrap the single query in a list.
    # scorer=Levenshtein.distance → same metric as ferrisfuzz (edit distance, not ratio).
    # Returns a 1×N int32 numpy matrix.
    return rf_process.cdist([query], candidates, scorer=RapidLevenshtein.distance)