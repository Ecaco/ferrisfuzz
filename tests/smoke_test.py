import ferrisfuzz

def test_surface():
    for name in ("levenshtein_bp", "damerau_bp", "jaro_winkler_similarity",
                 "levenshtein_batch", "damerau_batch", "jaro_winkler_batch"):
        assert hasattr(ferrisfuzz, name)

def test_known_values():
    assert ferrisfuzz.levenshtein_bp("kitten", "sitting") == 3
    assert ferrisfuzz.damerau_bp("teh", "the") == 1
    assert ferrisfuzz.jaro_winkler_similarity("", "") == 1.0
    assert ferrisfuzz.levenshtein_batch("kitten", ["sitting", "mitten"]) == [3, 1]