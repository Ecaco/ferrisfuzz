import ferrisfuzz
print(ferrisfuzz.damerau("MARTHA", "MARHTA"))
print(ferrisfuzz.levenshtein_distance("MARTHA", "MARHTA"))

SHORT_A = "MARTHA"
SHORT_B = "MARHTA"

LONG_A = "the quick brown fox jumps over the lazy dog"
LONG_B = "the slow green fox jumped over the lazy cat"


def pure_python_damerau(s1, s2):
    m, n = len(s1), len(s2)
    if m == 0: return n
    if n == 0: return m

    d = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1):
        d[i][0] = i
    for j in range(n + 1):
        d[0][j] = j

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            cost = 0 if s1[i - 1] == s2[j - 1] else 1
            d[i][j] = min(
                d[i - 1][j] + 1,      # deletion
                d[i][j - 1] + 1,      # insertion
                d[i - 1][j - 1] + cost # substitution
            )
            if i > 1 and j > 1 and s1[i - 1] == s2[j - 2] and s1[i - 2] == s2[j - 1]:
                d[i][j] = min(d[i][j], d[i - 2][j - 2] + cost) # transposition

    return d[m][n]

def test_rust_damerau_short(benchmark):
    benchmark(ferrisfuzz.damerau, SHORT_A, SHORT_B)

def test_python_damerau_short(benchmark):
    benchmark(pure_python_damerau, SHORT_A, SHORT_B)

def test_rust_damerau_long(benchmark):
    benchmark(ferrisfuzz.damerau, LONG_A, LONG_B)

def test_python_damerau_long(benchmark):
    benchmark(pure_python_damerau, LONG_A, LONG_B)


