import ferrisfuzz
from rapidfuzz.distance import Levenshtein as RapidLevenshtein

SHORT_A = "kitten"
SHORT_B = "sitting"
LONG_A = "the quick brown fox jumps over the lazy dog"
LONG_B = "the slow green fox jumped over the lazy cat"

def pure_python_levenshtein(s1, s2):
    m, n = len(s1), len(s2)
    dp = list(range(n + 1))
    for i in range(1, m + 1):
        prev = dp[0]
        dp[0] = i
        for j in range(1, n + 1):
            temp = dp[j]
            if s1[i-1] == s2[j-1]:
                dp[j] = prev
            else:
                dp[j] = 1 + min(prev, dp[j], dp[j-1])
            prev = temp
    return dp[n]

def pure_python_myers(s1, s2):
    m, n = len(s1), len(s2)
    if m == 0: return n
    if n == 0: return m
    max_edits = m + n
    offset = max_edits
    furthest = [0] * (2 * max_edits + 1)
    for k in range(max_edits + 1):
        for d in range(-k, k + 1, 2):
            if d == -k:
                row = furthest[d + 1 + offset]
            elif d == k:
                row = furthest[d - 1 + offset] + 1
            else:
                from_above = furthest[d - 1 + offset] + 1
                from_left = furthest[d + 1 + offset]
                row = max(from_above, from_left)
            col = row - d
            while row < m and col < n and s1[row] == s2[col]:
                row += 1
                col += 1
            furthest[d + offset] = row
            if row == m and col == n:
                return k
    return max_edits

# --- short strings ---

def test_rust_levenshtein_short(benchmark):
    benchmark(ferrisfuzz.levenshtein_distance, SHORT_A, SHORT_B)

def test_python_levenshtein_short(benchmark):
    benchmark(pure_python_levenshtein, SHORT_A, SHORT_B)

def test_rapidfuzz_levenshtein_short(benchmark):
    benchmark(RapidLevenshtein.distance, SHORT_A, SHORT_B)

def test_rust_myers_short(benchmark):
    benchmark(ferrisfuzz.myers_distance, SHORT_A, SHORT_B)

def test_python_myers_short(benchmark):
    benchmark(pure_python_myers, SHORT_A, SHORT_B)

# --- long strings ---

def test_rust_levenshtein_long(benchmark):
    benchmark(ferrisfuzz.levenshtein_distance, LONG_A, LONG_B)

def test_python_levenshtein_long(benchmark):
    benchmark(pure_python_levenshtein, LONG_A, LONG_B)

def test_rapidfuzz_levenshtein_long(benchmark):
    benchmark(RapidLevenshtein.distance, LONG_A, LONG_B)

def test_rust_myers_long(benchmark):
    benchmark(ferrisfuzz.myers_distance, LONG_A, LONG_B)

def test_python_myers_long(benchmark):
    benchmark(pure_python_myers, LONG_A, LONG_B)

    