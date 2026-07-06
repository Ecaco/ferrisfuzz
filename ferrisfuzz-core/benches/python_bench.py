"""
Python-side benchmarks for ferrisfuzz — measures the WHEEL (core + FFI + GIL),
not the core; criterion already covered that.

Rules: maturin develop --release first. Quote MINs. rapidfuzz = baseline,
pure-Python = floor. Batch sizes straddle DETACH_THRESHOLD (10k).


Run: pytest python_bench.py --benchmark-disable-gc --benchmark-sort=name
     (-k "not detach" to skip the GIL demo)
"""

import threading
import time

import pytest
import ferrisfuzz
from rapidfuzz.distance import Levenshtein as rf_lev
from rapidfuzz.distance import OSA as rf_osa
from rapidfuzz.distance import JaroWinkler as rf_jw
from rapidfuzz import process as rf_process

# --- surface check: fail loudly on a stale wheel -----------------------

_EXPECTED = [
    "levenshtein_bp", "damerau_bp", "jaro_winkler_similarity",
    "levenshtein_batch", "damerau_batch", "jaro_winkler_batch",
]
_missing = [n for n in _EXPECTED if not hasattr(ferrisfuzz, n)]
assert not _missing, (
    f"Wheel surface mismatch — missing {_missing}. Stale wheel? "
    f"Run `maturin develop --release`."
)

# --- inputs: mirror the Rust bench -------------------------------------

SHORT = ("kitten", "sitting")
MEDIUM = ("acknowledgement", "acknowledgments")
LONG = (
    "the quick brown fox jumps over the lazy dog",
    "the slow green fox jumped over the lazy cat",
)
PAIRS = {"short": SHORT, "medium": MEDIUM, "long": LONG}

QUERY = "kitten"
BATCH_SIZES = [1_000, 5_000, 50_000]


def make_candidates(n: int) -> list[str]:
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


CANDIDATES = {n: make_candidates(n) for n in BATCH_SIZES}


def py_levenshtein(s1, s2):
    m, n = len(s1), len(s2)
    if m == 0: return n
    if n == 0: return m
    prev = list(range(n + 1))
    for i in range(1, m + 1):
        cur = [i] + [0] * n
        for j in range(1, n + 1):
            cost = 0 if s1[i - 1] == s2[j - 1] else 1
            cur[j] = min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + cost)
        prev = cur
    return prev[n]


def py_osa(s1, s2):
    m, n = len(s1), len(s2)
    if m == 0: return n
    if n == 0: return m
    d = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1): d[i][0] = i
    for j in range(n + 1): d[0][j] = j
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            cost = 0 if s1[i - 1] == s2[j - 1] else 1
            d[i][j] = min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost)
            if i > 1 and j > 1 and s1[i-1] == s2[j-2] and s1[i-2] == s2[j-1]:
                d[i][j] = min(d[i][j], d[i - 2][j - 2] + 1)
    return d[m][n]


def py_jaro_winkler(s1, s2, p=0.1):
    # ORACLE — deliberately independent arithmetic. Does not share code with
    # the Rust core or call into it beyond the function under test. Semantics
    # match rapidfuzz: half-transpositions floored to whole transpositions
    # (integer division) before the float arithmetic, and the Winkler prefix
    # bonus applies only when raw Jaro strictly exceeds 0.7.
    m, n = len(s1), len(s2)
    if m == 0 and n == 0: return 1.0
    if m == 0 or n == 0: return 0.0
    window = max((max(m, n) // 2) - 1, 0)
    s1_m = [False] * m
    s2_m = [False] * n
    matches = 0
    for i in range(m):
        start = max(0, i - window)
        end = min(i + window + 1, n)
        for j in range(start, end):
            if s2_m[j] or s1[i] != s2[j]:
                continue
            s1_m[i] = s2_m[j] = True
            matches += 1
            break
    if matches == 0: return 0.0
    half_t = 0
    k = 0
    for i in range(m):
        if not s1_m[i]: continue
        while not s2_m[k]: k += 1
        if s1[i] != s2[k]: half_t += 1
        k += 1
    t = half_t // 2   # floor BEFORE the float arithmetic, matching rapidfuzz
    jaro = (matches / m + matches / n + (matches - t) / matches) / 3.0
    prefix = 0
    for i in range(min(4, m, n)):
        if s1[i] == s2[i]: prefix += 1
        else: break
    if jaro > 0.7:    # Winkler boost threshold: raw jaro, strictly greater
        return jaro + prefix * p * (1.0 - jaro)
    return jaro


# --- parity gates: no timings from a wrong build ------------------------

PARITY_PAIRS = [
    ("kitten", "sitting"), ("MARTHA", "MARHTA"), ("abc", "bca"),
    ("teh", "the"), ("acknowledgement", "acknowledgments"),
    LONG, ("", ""), ("a", ""),
]


@pytest.mark.parametrize("a,b", PARITY_PAIRS)
def test_parity_levenshtein(a, b):
    ours = ferrisfuzz.levenshtein_bp(a, b)
    assert ours == py_levenshtein(a, b)
    assert ours == rf_lev.distance(a, b)


@pytest.mark.parametrize("a,b", PARITY_PAIRS)
def test_parity_osa(a, b):
    ours = ferrisfuzz.damerau_bp(a, b)
    assert ours == py_osa(a, b)
    assert ours == rf_osa.distance(a, b)


# HARD GATE: our build vs our own semantics. Must always pass.
@pytest.mark.parametrize("a,b", PARITY_PAIRS)
def test_parity_jaro_winkler(a, b):
    ours = ferrisfuzz.jaro_winkler_similarity(a, b)
    assert ours == pytest.approx(py_jaro_winkler(a, b), abs=1e-9)



@pytest.mark.parametrize("a,b", PARITY_PAIRS)
def test_parity_jaro_winkler_vs_rapidfuzz(a, b):
    ours = ferrisfuzz.jaro_winkler_similarity(a, b)
    assert ours == pytest.approx(rf_jw.similarity(a, b), abs=1e-9)


def test_parity_batches():
    cands = CANDIDATES[1_000]
    assert ferrisfuzz.levenshtein_batch(QUERY, cands) == \
        [ferrisfuzz.levenshtein_bp(QUERY, c) for c in cands]
    assert ferrisfuzz.damerau_batch(QUERY, cands) == \
        [ferrisfuzz.damerau_bp(QUERY, c) for c in cands]
    jw_b = ferrisfuzz.jaro_winkler_batch(QUERY, cands)
    jw_s = [ferrisfuzz.jaro_winkler_similarity(QUERY, c) for c in cands]
    for b, s in zip(jw_b, jw_s):
        assert b == pytest.approx(s, abs=1e-9)


# --- single-pair benches: pedantic beats the ~100ns timer grid ----------

def _pedantic(benchmark, fn, *args):
    benchmark.pedantic(fn, args=args, iterations=1000, rounds=200)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_lev_ours(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, ferrisfuzz.levenshtein_bp, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_lev_rapidfuzz(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, rf_lev.distance, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_lev_python(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, py_levenshtein, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_osa_ours(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, ferrisfuzz.damerau_bp, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_osa_rapidfuzz(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, rf_osa.distance, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_osa_python(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, py_osa, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_jw_ours(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, ferrisfuzz.jaro_winkler_similarity, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_jw_rapidfuzz(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, rf_jw.similarity, a, b)


@pytest.mark.parametrize("label", PAIRS)
def test_bench_jw_python(benchmark, label):
    a, b = PAIRS[label]
    _pedantic(benchmark, py_jaro_winkler, a, b)


# --- batch benches -------------------------------------------------------

@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_lev_ours(benchmark, n):
    benchmark(ferrisfuzz.levenshtein_batch, QUERY, CANDIDATES[n])

@pytest.mark.benchmark(warmup=True)
@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_lev_cdist(benchmark, n):
    benchmark(rf_process.cdist, [QUERY], CANDIDATES[n], scorer=rf_lev.distance)


@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_osa_ours(benchmark, n):
    benchmark(ferrisfuzz.damerau_batch, QUERY, CANDIDATES[n])


@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_osa_cdist(benchmark, n):
    benchmark(rf_process.cdist, [QUERY], CANDIDATES[n], scorer=rf_osa.distance)


@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_jw_ours(benchmark, n):
    benchmark(ferrisfuzz.jaro_winkler_batch, QUERY, CANDIDATES[n])


@pytest.mark.parametrize("n", BATCH_SIZES)
def test_bench_batch_jw_cdist(benchmark, n):
    benchmark(rf_process.cdist, [QUERY], CANDIDATES[n], scorer=rf_jw.similarity)


# pure-Python floor: 1k only — context, not signal
def test_bench_batch_lev_python(benchmark):
    cands = CANDIDATES[1_000]
    benchmark(lambda: [py_levenshtein(QUERY, c) for c in cands])

def _burn(stop, counter):
    while not stop.is_set():
        x = 0
        for _ in range(10_000):
            x += 1
        counter[0] += 1


def _count_burns_during(fn, duration_hint=0.5):
    stop = threading.Event()
    counter = [0]
    t = threading.Thread(target=_burn, args=(stop, counter))
    t.start()
    time.sleep(0.05)
    start = time.perf_counter()
    while time.perf_counter() - start < duration_hint:
        fn()
    stop.set()
    t.join()
    return counter[0]


def test_detach_composes_with_threads():
    big = CANDIDATES[50_000]      # above gate: GIL released
    small = CANDIDATES[1_000]     # below gate: GIL held

    burns_released = _count_burns_during(
        lambda: ferrisfuzz.levenshtein_batch(QUERY, big))
    burns_held = _count_burns_during(
        lambda: [ferrisfuzz.levenshtein_batch(QUERY, small) for _ in range(50)])

    assert burns_released > burns_held * 1.5, (
        f"GIL release not observable: released={burns_released}, "
        f"held={burns_held}. Check DETACH_THRESHOLD and detach() wiring."
    )

_PROBES = [
    ("borrow", "_ingest_borrow"),                   
    ("borrow_deref", "_ingest_borrow_and_deref"),   
    ("owned", "_ingest_owned"),                     
]

_have_probes = all(hasattr(ferrisfuzz, fn) for _, fn in _PROBES)


@pytest.mark.skipif(not _have_probes, reason="_ingest_* probes not in wheel")
@pytest.mark.benchmark(warmup=True)
@pytest.mark.parametrize("n", BATCH_SIZES)
@pytest.mark.parametrize("label,fn_name", _PROBES)
def test_probe_ingest(benchmark, label, fn_name, n):
    fn = getattr(ferrisfuzz, fn_name)
    benchmark(fn, CANDIDATES[n])