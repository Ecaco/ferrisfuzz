use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::format;

use crate::common::{normalize, check_len, apply_cutoff, MatchError};

/// Bit-parallel Levenshtein (Myers' algorithm).
///
/// Every option below is opt-in and costs nothing on the default path:
/// - `max_len`: reject inputs longer than this many chars (`None` ⇒ no cap).
/// - `case_insensitive`: lowercase both inputs once, up front (`None` ⇒ false, borrows).
/// - `score_cutoff`: if the distance would exceed this, return `cutoff + 1` and,
///   where provable up front, skip the computation entirely (`None` ⇒ exact distance).


pub fn levenshtein_bp(
    str_1: &str,
    str_2: &str,
    max_len: Option<usize>,
    case_insensitive: Option<bool>,
    score_cutoff: Option<usize>,
) -> Result<usize, MatchError> {
    let s1 = normalize(str_1, case_insensitive);
    let s2 = normalize(str_2, case_insensitive);

    let ascii_only = s1.is_ascii() && s2.is_ascii();

    // On the ASCII path, byte length == char length — so m/n need no decode.
    // Off it, count chars (still once each).
    let (m, n) = if ascii_only {
        (s1.len(), s2.len())
    } else {
        (s1.chars().count(), s2.chars().count())
    };

    check_len(m, max_len, "str_1")?;
    check_len(n, max_len, "str_2")?;

    if let Some(k) = score_cutoff {
        if m.abs_diff(n) > k {
            return Ok(k + 1);
        }
    }

    if m == 0 { return Ok(apply_cutoff(n, score_cutoff)); }
    if n == 0 { return Ok(apply_cutoff(m, score_cutoff)); }

    let score = if ascii_only && m <= 64 {
        // no Vec<char> built at all on this path
        levenshtein_bp_small_bytes(s1.as_bytes(), s2.as_bytes(), m)
    } else {
        // only the char paths pay for the collect
        let query: Vec<char> = s1.chars().collect();
        if m <= 64 {
            levenshtein_bp_small(&query, s2.as_ref(), m)
        } else {
            levenshtein_bp_multiword(&query, s2.as_ref(), m)
        }
    };

    Ok(apply_cutoff(score, score_cutoff))
}

fn levenshtein_bp_small(query: &[char], target: &str, m: usize) -> usize {
    // Direct-indexed match vectors for chars < 256; BTreeMap fallback beyond.
    let mut ascii = [0u64; 256];
    let mut wide: BTreeMap<char, u64> = BTreeMap::new();
    for (i, &c) in query.iter().enumerate() {
        let cp = c as u32;
        if cp < 256 {
            ascii[cp as usize] |= 1u64 << i;
        } else {
            *wide.entry(c).or_insert(0) |= 1u64 << i;
        }
    }

    let mut pv: u64 = u64::MAX;
    let mut mv: u64 = 0;
    let mut score = m;
    let mask = 1u64 << (m - 1);

    for c in target.chars() {
        let cp = c as u32;
        let eq = if cp < 256 {
            ascii[cp as usize]
        } else {
            wide.get(&c).copied().unwrap_or(0)
        };

        let xh = ((eq & pv).wrapping_add(pv)) ^ pv;
        let x  = xh | eq;
        let ph = mv | !(x | pv);
        let mh = pv & x;
        if ph & mask != 0 { score += 1; }
        if mh & mask != 0 { score -= 1; }
        let ph_shift = (ph << 1) | 1;
        let mh_shift = mh << 1;
        pv = mh_shift | !(x | ph_shift);
        mv = ph_shift & x;
    }
    score
}

fn levenshtein_bp_multiword(query: &[char], target: &str, m: usize) -> usize {
    let num_words = (m + 63) / 64;
    let last_mask = 1u64 << ((m - 1) % 64);

    let mut ascii = vec![0u64; 256 * num_words];
    let mut wide: BTreeMap<char, Vec<u64>> = BTreeMap::new();
    for (i, &c) in query.iter().enumerate() {
        let word = i / 64;
        let bit  = i % 64;
        let cp = c as u32;
        if cp < 256 {
            ascii[cp as usize * num_words + word] |= 1u64 << bit;
        } else {
            wide.entry(c)
                .or_insert_with(|| vec![0u64; num_words])[word] |= 1u64 << bit;
        }
    }

    let mut pv = vec![u64::MAX; num_words];
    let mut mv = vec![0u64;     num_words];
    let mut score = m;

    let empty = vec![0u64; num_words];
    let mut xh = vec![0u64; num_words];
    let mut ph = vec![0u64; num_words];
    let mut mh = vec![0u64; num_words];

    for c in target.chars() {
        let cp = c as u32;
        let eq: &[u64] = if cp < 256 {
            let base = cp as usize * num_words;
            &ascii[base..base + num_words]
        } else {
            wide.get(&c).map(|v| v.as_slice()).unwrap_or(&empty)
        };

        let mut carry = 0u64;
        for i in 0..num_words {
            let x_i = eq[i] & pv[i];
            let (s1, c1) = x_i.overflowing_add(pv[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            carry = (c1 || c2) as u64;
            xh[i] = s2 ^ pv[i];
        }

        for i in 0..num_words {
            let x_i = xh[i] | eq[i];
            ph[i] = mv[i] | !(x_i | pv[i]);
            mh[i] = pv[i] & x_i;
        }

        let last = num_words - 1;
        if ph[last] & last_mask != 0 { score += 1; }
        if mh[last] & last_mask != 0 { score = score.saturating_sub(1); }

        let mut carry_ph = 1u64;
        let mut carry_mh = 0u64;
        for i in 0..num_words {
            let x_i      = xh[i] | eq[i];
            let ph_shift = (ph[i] << 1) | carry_ph;
            let mh_shift = (mh[i] << 1) | carry_mh;
            carry_ph     = ph[i] >> 63;
            carry_mh     = mh[i] >> 63;
            pv[i] = mh_shift | !(x_i | ph_shift);
            mv[i] = ph_shift & x_i;
        }
    }

    score
}

fn levenshtein_bp_small_bytes(query: &[u8], target: &[u8], m: usize) -> usize {
    // No `wide` fallback and no `< 256` branch: every byte indexes directly.
    let mut peq = [0u64; 256];
    for (i, &b) in query.iter().enumerate() {
        peq[b as usize] |= 1u64 << i;
    }
 
    let mut pv: u64 = u64::MAX;
    let mut mv: u64 = 0;
    let mut score = m;
    let mask = 1u64 << (m - 1);
 
    for &b in target.iter() {
        let eq = peq[b as usize]; // single load, no decode, no branch
 
        let xh = ((eq & pv).wrapping_add(pv)) ^ pv;
        let x  = xh | eq;
        let ph = mv | !(x | pv);
        let mh = pv & x;
        if ph & mask != 0 { score += 1; }
        if mh & mask != 0 { score -= 1; }
        let ph_shift = (ph << 1) | 1;
        let mh_shift = mh << 1;
        pv = mh_shift | !(x | ph_shift);
        mv = ph_shift & x;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levenshtein::levenshtein_distance;

    #[test]
    fn test_basic() {
        assert_eq!(levenshtein_bp("kitten", "sitting", None, None, None), Ok(3));
    }

    #[test]
    fn test_case_insensitive() {
        let ci = Some(true);
        assert_eq!(levenshtein_bp("Kitten", "KITTEN", None, ci, None), Ok(0));
        assert_eq!(levenshtein_bp("HELLO", "hello", None, ci, None), Ok(0));
        // case-sensitive default still counts the case difference
        assert_eq!(levenshtein_bp("Kitten", "KITTEN", None, None, None), Ok(5));
    }

    #[test]
    fn test_max_len_rejects() {
        let long = "a".repeat(50);
        let err = levenshtein_bp(&long, "a", Some(10), None, None);
        assert!(matches!(err, Err(MatchError::InputTooLong { .. })));
    }

    #[test]
    fn test_score_cutoff() {
        // within cutoff → exact distance
        assert_eq!(levenshtein_bp("kitten", "sitting", None, None, Some(5)), Ok(3));
        // exceeds cutoff → cutoff + 1 (not the exact distance)
        assert_eq!(levenshtein_bp("kitten", "sitting", None, None, Some(2)), Ok(3));
        // length pre-filter: |m - n| already exceeds cutoff → cutoff + 1, no compute
        assert_eq!(levenshtein_bp("a", "abcdefgh", None, None, Some(3)), Ok(4));
    }

    #[test]
    fn test_cutoff_matches_exact_when_within() {
        // For any pair, if cutoff >= true distance, cutoff result == exact result.
        let pairs = [("flaw", "lawn"), ("gumbo", "gambol"), ("Saturday", "Sunday")];
        for (a, b) in pairs {
            let exact = levenshtein_bp(a, b, None, None, None).unwrap();
            let capped = levenshtein_bp(a, b, None, None, Some(exact)).unwrap();
            assert_eq!(exact, capped, "cutoff changed a within-bound result for {a:?}/{b:?}");
        }
    }

    #[test]
    fn test_crosscheck() {
        let pairs = [
            ("", ""), ("a", ""), ("", "a"), ("a", "a"), ("a", "b"),
            ("kitten", "sitting"), ("flaw", "lawn"), ("gumbo", "gambol"),
            ("Saturday", "Sunday"),
            ("the quick brown fox", "the slow green fox"),
        ];
        for (s1, s2) in pairs {
            let bp = levenshtein_bp(s1, s2, None, None, None);
            let classic = levenshtein_distance(s1, s2, None, None).unwrap();
            assert_eq!(bp, Ok(classic), "mismatch on {:?} vs {:?}", s1, s2);
        }
    }

    #[test]
    fn test_crosscheck_multiword() {
        let pairs = [
            ("a".repeat(70), "a".repeat(70)),
            ("a".repeat(70), "b".repeat(70)),
        ];
        for (s1, s2) in &pairs {
            let bp = levenshtein_bp(s1, s2, None, None, None);
            let classic = levenshtein_distance(s1, s2, None, None).unwrap();
            assert_eq!(bp, Ok(classic), "mismatch len {}", s1.chars().count());
        }
    }
    #[test]
    fn test_bytes_matches_chars() {
        // ASCII: fast path and char path must agree exactly
        for (a, b) in [("kitten","sitting"), ("flaw","lawn"), ("Saturday","Sunday")] {
            // (both go through the same public fn; this proves the ascii fork is correct)
            let d = levenshtein_bp(a, b, None, None, None).unwrap();
            let classic = levenshtein_distance(a, b, None, None).unwrap();
            assert_eq!(d, classic);
        }
    }

    #[test]
    fn test_non_ascii_takes_char_path_and_is_correct() {
        // café vs cafe: 1 edit at the CHARACTER level. Byte-Myers would say 2 (é = 2 bytes).
        // This test fails loudly if a non-ASCII input wrongly hits the bytes path.
        let d = levenshtein_bp("café", "cafe", None, None, None).unwrap();
        let classic = levenshtein_distance("café", "cafe", None, None).unwrap();
        assert_eq!(d, classic, "non-ASCII must use the char path");
        assert_eq!(d, 1);
    }
}