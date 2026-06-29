import ferrisfuzz
print(ferrisfuzz.jaro_winkler_distance("MARTHA", "MARHTA"))
print(ferrisfuzz.jaro_winkler_distance("MARTHA", "MARHTA", p=0.0))
print(ferrisfuzz.jaro_winkler_distance("CRATE", "trace", case_insensitive=True))

SHORT_A = "MARTHA"
SHORT_B = "MARHTA"

LONG_A = "the quick brown fox jumps over the lazy dog"
LONG_B = "the slow green fox jumped over the lazy cat"

def pure_python_jaro_winkler(s1, s2, p=0.1):
    m, n = len(s1), len(s2)
    if m == 0 or n == 0:
        return 0.0

    match_distance = (max(m, n) // 2) - 1
    matches = 0
    transpositions = 0
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

    transpositions //= 2

    jaro_distance = (matches / m + matches / n + (matches - transpositions) / matches) / 3
    prefix_length = 0
    for i in range(min(4, m, n)):
        if s1[i] == s2[i]:
            prefix_length += 1
        else:
            break
    jaro_winkler_distance = jaro_distance + (prefix_length * p * (1 - jaro_distance))
    return jaro_winkler_distance

def test_rust_jaro_winkler_short(benchmark):
    benchmark(ferrisfuzz.jaro_winkler_distance, SHORT_A, SHORT_B)

def test_python_jaro_winkler_short(benchmark):
    benchmark(pure_python_jaro_winkler, SHORT_A, SHORT_B)

def test_rust_jaro_winkler_long(benchmark):
    benchmark(ferrisfuzz.jaro_winkler_distance, LONG_A, LONG_B)

def test_python_jaro_winkler_long(benchmark):
    benchmark(pure_python_jaro_winkler, LONG_A, LONG_B)

