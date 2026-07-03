import pytest
import ferrisfuzz
from rapidfuzz.distance import JaroWinkler as rf_jw
from rapidfuzz import process as rf_process

# ----------------------------------------------------------------------
# Fixtures / inputs
# ----------------------------------------------------------------------

SHORT_A = "MARTHA"
SHORT_B = "MARHTA"

LONG_A = "the quick brown fox jumps over the lazy dog"
LONG_B = "the slow green fox jumped over the lazy cat"

QUERY = "kitten"


def make_candidates(n: int) -> list[str]:
    """Mirror of the Rust bench's make_candidates: near-query ASCII words,
    with occasional adjacent swaps so the transposition machinery works."""
    bases = ["kitten", "sitting", "mitten", "kitchen", "bitten", "written"]
    out = []
    for i in range(n):
        w = bases[i % len(bases)]
        if i % 3 == 0:
            w += chr(ord("a") + i % 26)
        if i % 5 == 0 and len(w) >= 3:
            w = w[0] + w[2] + w[1] + w[3:]
        out.append(w)
    return out


# Sizes chosen to straddle DETACH_THRESHOLD = 10_000:
# 1k and 5k hold the GIL; 50k pays the detach tax and should amortize it.
BATCH_SIZES = [1_000, 5_000, 50_000]
CANDIDATES = {n: make_candidates(n) for n in BATCH_SIZES}


# ----------------------------------------------------------------------
# Pure-Python reference (the floor, and a semantics oracle)
# ----------------------------------------------------------------------

def pure_python_jaro_winkler(s1, s2, p=0.1):
    m, n = len(s1), len(s2)
    if m == 0 and n == 0:
        return 1.0          # match the core's empty-string semantics
    if m == 0 or n == 0:
        return 0.0

    match_distance = max((max(m, n) // 2) - 1, 0)
    matches = 0
    transpositions = 0      # counted as HALF-transpositions, like the core
    s1_matches = [False] * m
    s2_matches = [False] * n

    for i in range(m):
        start = max(0, i - match_distance)
        end = min(i + match_distance + 1, n)
        for j in range(start, end):
            if s2_matches[j]:
                continue
            if s1[i] != s2[j]:
                continue
            s1_matches[i] = True
            s2_matches[j] = True
            matches += 1
            break

    if matches == 0:
        return 0.0

    k = 0
    for i in range(m):
        if not s1_matches[i]:
            continue
        while not s2_matches[k]:
            k += 1
        if s1[i] != s2[k]:
            transpositions += 1
        k += 1

    # EXACT halving — no floor. The old version did `transpositions //= 2`,
    # which disagrees with the Rust core (and rapidfuzz) whenever the
    # half-transposition count is odd, e.g. ("abc", "bca").
    jaro = (matches / m + matches / n
            + (matches - transpositions / 2.0) / matches) / 3.0

    prefix = 0
    for i in range(min(4, m, n)):
        if s1[i] == s2[i]:
            prefix += 1
        else:
            break
    return jaro + prefix * p * (1.0 - jaro)


def pure_python_batch(query, candidates, p=0.1):
    return [pure_python_jaro_winkler(query, c, p) for c in candidates]


# ----------------------------------------------------------------------
# Parity smoke tests — the gate before any timing is believed.
# Three-way agreement: ferrisfuzz == rapidfuzz == pure-Python reference.
# ----------------------------------------------------------------------

PARITY_PAIRS = [
    ("MARTHA", "MARHTA"),
    ("DWAYNE", "DUANE"),
    ("DIXON", "DICKSONX"),
    ("abc", "bca"),        # odd half-transposition count — the floor-bug detector
    ("kitten", "sitting"),
    ("", ""),
    ("a", ""),
]


@pytest.mark.parametrize("a,b", PARITY_PAIRS)
def test_parity_three_way(a, b):
    ours = ferrisfuzz.jaro_winkler_similarity(a, b)
    ref = pure_python_jaro_winkler(a, b)
    rf = rf_jw.similarity(a, b)
    assert ours == pytest.approx(ref, abs=1e-9), f"{a!r}/{b!r} ours vs pure"
    assert ours == pytest.approx(rf, abs=1e-9), f"{a!r}/{b!r} ours vs rapidfuzz"


def test_parity_options():
    # p=0 kills the prefix bonus → plain Jaro
    assert ferrisfuzz.jaro_winkler_similarity("MARTHA", "MARHTA", p=0.0) == \
        pytest.approx(pure_python_jaro_winkler("MARTHA", "MARHTA", p=0.0), abs=1e-9)
    # case-insensitive
    assert ferrisfuzz.jaro_winkler_similarity("CRATE", "trace", case_insensitive=True) == \
        pytest.approx(pure_python_jaro_winkler("crate", "trace"), abs=1e-9)
    # similarity FLOOR: below cutoff reports 0.0
    assert ferrisfuzz.jaro_winkler_similarity("abc", "xyz", score_cutoff=0.9) == 0.0


def test_parity_batch():
    cands = CANDIDATES[1_000]
    ours = ferrisfuzz.jaro_winkler_batch(QUERY, cands)
    ref = pure_python_batch(QUERY, cands)
    for o, r, w in zip(ours, ref, cands):
        assert o == pytest.approx(r, abs=1e-9), f"batch mismatch on {w!r}"


# ----------------------------------------------------------------------
# Single-pair benchmarks
# ----------------------------------------------------------------------

def test_bench_single_short_ours(benchmark):
    benchmark.pedantic(ferrisfuzz.jaro_winkler_similarity,
                       args=(SHORT_A, SHORT_B),
                       iterations=1000, rounds=200)

def test_bench_single_short_rapidfuzz(benchmark):
    benchmark.pedantic(rf_jw.similarity,
                       args=(SHORT_A, SHORT_B),
                       iterations=1000, rounds=200)

def test_bench_single_short_python(benchmark):
    benchmark.pedantic(pure_python_jaro_winkler,
                       args=(SHORT_A, SHORT_B),
                       iterations=1000, rounds=200)

def test_bench_single_long_ours(benchmark):
    benchmark.pedantic(ferrisfuzz.jaro_winkler_similarity,
                       args=(LONG_A, LONG_B),
                       iterations=1000, rounds=200)

def test_bench_single_long_rapidfuzz(benchmark):
    benchmark.pedantic(rf_jw.similarity,
                       args=(LONG_A, LONG_B),
                       iterations=1000, rounds=200)

def test_bench_single_long_python(benchmark):
    benchmark.pedantic(pure_python_jaro_winkler,
                       args=(LONG_A, LONG_B),
                       iterations=1000, rounds=200)


# ----------------------------------------------------------------------
# Batch benchmarks — sized to expose the detach-gating crossover
# ----------------------------------------------------------------------

@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_ours(benchmark, n):
    cands = CANDIDATES[n]
    benchmark(ferrisfuzz.jaro_winkler_batch, QUERY, cands)

@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_rapidfuzz_cdist(benchmark, n):
    cands = CANDIDATES[n]
    benchmark(rf_process.cdist, [QUERY], cands, scorer=rf_jw.similarity)

@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_python(benchmark, n):
    cands = CANDIDATES[n]
    benchmark(pure_python_batch, QUERY, cands)