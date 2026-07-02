"""
FINAL AUTOPSY — locating the residual ~226us between borrow-ingestion (~117us)
and the full scalar-return batch call (~343us).

Everything runs in ONE invocation so the subtractions share machine state.
Read the MIN column. Each pair below isolates exactly one stage:

  detach cost      = sum        - sum_nodetach
  deref/&str build = ingest_deref - ingest_borrow
  core compute     = sum_nodetach - ingest_deref     (what's left = the real loop)

Run:
  pytest test_boundary_autopsy.py --benchmark-min-rounds=50 -q --benchmark-sort=min
"""

import ferrisfuzz
from rapidfuzz.distance import Levenshtein as RapidLevenshtein
from rapidfuzz import process as rf_process

def _cdist_numpy(query, candidates):
    # single query wrapped in a list; scorer = same edit-distance metric as ferrisfuzz
    return rf_process.cdist([query], candidates, scorer=RapidLevenshtein.distance)

N = 10_000
QUERY = "kitten"
_BASES = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written", "smitten", "cat"]
CANDIDATES = [_BASES[i % len(_BASES)] + ("s" if i % 3 == 0 else "") for i in range(N)]


# --- anchors we already measured (kept here so the whole ladder is in one run) ---

def test_a_ingest_borrow(benchmark):
    # just candidates.len() — pure crossing, no deref, no compute. (~117us baseline)
    benchmark(ferrisfuzz._ingest_borrow, CANDIDATES)


def test_b_ingest_borrow_and_deref(benchmark):
    # PROBE 2: builds Vec<&str> from the PyBackedStr handles and touches each,
    # but runs NO edit distance. Difference vs (a) = cost of materializing &str.
    benchmark(ferrisfuzz._ingest_borrow_and_deref, CANDIDATES)


# --- the two full-work variants: same everything, GIL released vs held ---



def test_d_sum_detach(benchmark):
    # PROBE 1a: identical to (c) but with detach() releasing the GIL.
    # Difference (d - c) = the cost of the detach boundary itself.
    benchmark(ferrisfuzz.levenshtein_batch_sum, QUERY, CANDIDATES)


# --- the real thing, for reference: full work + build the 10k Python list ---

def test_e_full_list(benchmark):
    # (e - d) = output list-building cost (we expect ~27us from last run).
    benchmark(ferrisfuzz.levenshtein_batch, QUERY, CANDIDATES)


# --- correctness guard: the scalar variants must sum the REAL distances ---
# (a fast wrong answer is still wrong — verify before trusting any timing)

def test_z_scalar_variants_agree():
    expected = sum(ferrisfuzz.levenshtein_batch(QUERY, CANDIDATES))
    assert ferrisfuzz.levenshtein_batch_sum(QUERY, CANDIDATES) == expected

CANDIDATES_5K = [_BASES[i % len(_BASES)] + ("s" if i % 3 == 0 else "") for i in range(5_000)]

def test_batch_5k_ferrisfuzz(benchmark):
    benchmark(ferrisfuzz.levenshtein_batch, QUERY, CANDIDATES_5K)

def test_batch_5k_cdist(benchmark):
    benchmark(_cdist_numpy, QUERY, CANDIDATES_5K)