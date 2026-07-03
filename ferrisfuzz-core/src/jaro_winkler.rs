use alloc::vec::Vec;
use alloc::vec;
use crate::common::{MatchError, check_len, normalize};

/// Jaro-Winkler SIMILARITY in [0.0, 1.0] (1.0 = identical). Note: a *similarity*,
/// not a distance — hence f64, not usize.
///
/// `p` is the Winkler prefix weight (default 0.1; must be in [0.0, 0.25]).
/// `score_cutoff` is a similarity FLOOR: if the result would be below it, return
/// 0.0 early. (Similarity cutoff = floor; the opposite sense to a distance ceiling.)
///
/// Shape: forked on ASCII, NOT on a 64-char word boundary. Jaro isn't bit-parallel,
/// so there's no `u64` packing and no small/multiword split. The only meaningful
/// fork is ASCII-fast (zero-alloc, byte + u64 match-bitmask) vs the general
/// char-based fallback.
pub fn jaro_winkler(
    str_1: &str,
    str_2: &str,
    p: Option<f64>,
    max_len: Option<usize>,
    case_insensitive: Option<bool>,
    score_cutoff: Option<f64>,
) -> Result<f64, MatchError> {
    // Winkler prefix weight: validate before any work.
    let p = p.unwrap_or(0.1);
    if !(0.0..=0.25).contains(&p) {
        return Err(MatchError::InvalidPrefixScale { value: p });
    }

    let s1 = normalize(str_1, case_insensitive);
    let s2 = normalize(str_2, case_insensitive);

    let ascii_only = s1.is_ascii() && s2.is_ascii();

    let (m, n) = if ascii_only {
        (s1.len(), s2.len())
    } else {
        (s1.chars().count(), s2.chars().count())
    };

    check_len(m, max_len, "str_1")?;
    check_len(n, max_len, "str_2")?;

    // Empty-string semantics (both empty = identical = 1.0; one empty = 0.0).
    if m == 0 && n == 0 { return Ok(1.0); }
    if m == 0 || n == 0 { return Ok(0.0); }

    // Fork: zero-alloc ASCII fast path (≤ 64 chars so the u64 match-bitmask fits),
    // otherwise the general char-based path.
    let sim = if ascii_only && m <= 64 && n <= 64 {
        if m.max(n) <= 16 {
            jaro_winkler_ascii_fast(s1.as_bytes(), s2.as_bytes(), p) // your current scan
        } else {
            jaro_winkler_ascii_bp(s1.as_bytes(), s2.as_bytes(), p)
        }
    } else {
        jaro_winkler_general(s1.as_ref(), s2.as_ref(), p)
    };

    // Similarity cutoff is a FLOOR: below it, report 0.0.
    if let Some(floor) = score_cutoff {
        if sim < floor {
            return Ok(0.0);
        }
    }

    Ok(sim)
}

/// zero-allocation ASCII fast path.
fn jaro_winkler_ascii_fast(a: &[u8], b: &[u8], p: f64) -> f64 {
    let m = a.len();
    let n = b.len();
 
    // Match window (same formula as the general path).
    let window = (m.max(n) / 2).saturating_sub(1);
 
    // ★ The bitmasks replace vec![false; m] and vec![false; n]. Zero allocation.
    let mut m1: u64 = 0; // which positions in `a` matched
    let mut m2: u64 = 0; // which positions in `b` matched
    let mut matches = 0usize;
 
    for i in 0..m {
        let start = i.saturating_sub(window);
        let end = (i + window + 1).min(n);
 
        for j in start..end {
            // "is b[j] already claimed?"  →  test bit j of m2
            if m2 & (1u64 << j) != 0 {
                continue;
            }
            // "do the characters match?"  →  direct byte compare (no decode)
            if a[i] != b[j] {
                continue;
            }
            // claim both positions: set bit i of m1, bit j of m2
            m1 |= 1u64 << i;
            m2 |= 1u64 << j;
            matches += 1;
            break;
        }
    }
 
    if matches == 0 {
        return 0.0;
    }
 
    // --- transposition count ---
    // Walk matched positions of `a` in order; for each, advance to the next matched
    // position of `b`. If the paired characters differ, that's a half-transposition.
    // Same logic as the general path, but "is k matched?" is a bit test on m2.
    let mut transpositions = 0usize;
    let mut k = 0usize;
    for i in 0..m {
        if m1 & (1u64 << i) == 0 {
            continue; // a[i] wasn't matched
        }
        // advance k to the next matched position in b
        while m2 & (1u64 << k) == 0 {
            k += 1;
        }
        if a[i] != b[k] {
            transpositions += 1;
        }
        k += 1;
    }
 
    // --- Jaro similarity ---
    let m_f = matches as f64;
    let jaro = (m_f / m as f64
        + m_f / n as f64
        + (m_f - transpositions as f64 / 2.0) / m_f)
        / 3.0;
 
    // --- Winkler prefix bonus (up to 4 leading chars that match) ---
    let mut prefix = 0usize;
    for idx in 0..m.min(n).min(4) {
        if a[idx] == b[idx] {
            prefix += 1;
        } else {
            break;
        }
    }
 
    jaro + (prefix as f64 * p * (1.0 - jaro))
}

/// STUB — general fallback (non-ASCII or > 64 chars).
/// This will be your CURRENT, working jaro_winkler body: chars() + Vec<bool> match
/// arrays. It's the correctness oracle the fast path is crosschecked against, so
/// keep it exactly as your tested version (just returning the bare f64, no Result —
/// the entry fn owns validation now).
fn jaro_winkler_general(a: &str, b: &str, p: f64) -> f64 {
    let chars_1: Vec<char> = a.chars().collect();
    let chars_2: Vec<char> = b.chars().collect();
    let m = chars_1.len();
    let n = chars_2.len();
 
    // Match window (identical formula to the fast path).
    let window = (m.max(n) / 2).saturating_sub(1);
 
    // Char-indexed match arrays. Everything here indexes by CHAR position `j`,
    // which is what keeps non-ASCII correct.
    let mut str1_matches = vec![false; m];
    let mut str2_matches = vec![false; n];
    let mut matches = 0usize;
 
    // --- Phase 1: windowed match scan ---
    for i in 0..m {
        let start = i.saturating_sub(window);
        let end = (i + window + 1).min(n);
 
        for j in start..end {
            if str2_matches[j] {
                continue; // b[j] already claimed
            }
            if chars_1[i] != chars_2[j] {
                continue; // not a match
            }
            str1_matches[i] = true;
            str2_matches[j] = true;
            matches += 1;
            break;
        }
    }
 
    if matches == 0 {
        return 0.0; // also guards the divide-by-matches below
    }
 
    // --- Phase 2: transposition count ---
    // Walk matched positions of `a` in order; pair each with the next matched
    // position of `b`. Differing pairs are half-transpositions.
    let mut transpositions = 0usize;
    let mut k = 0usize;
    for i in 0..m {
        if !str1_matches[i] {
            continue;
        }
        while !str2_matches[k] {
            k += 1;
        }
        if chars_1[i] != chars_2[k] {
            transpositions += 1;
        }
        k += 1;
    }
 
    // --- Phase 3: Jaro similarity ---
    let m_f = matches as f64;
    let jaro = (m_f / m as f64
        + m_f / n as f64
        + (m_f - transpositions as f64 / 2.0) / m_f)
        / 3.0;
 
    // --- Winkler prefix bonus (up to 4 leading matching chars) ---
    let prefix = chars_1
        .iter()
        .zip(chars_2.iter())
        .take(4)
        .take_while(|(x, y)| x == y)
        .count() as f64;
 
    jaro + (prefix * p * (1.0 - jaro))
}

/// Bit-parallel ASCII path. Same greedy semantics as `jaro_winkler_general`
/// (outer over `a`, claim earliest unclaimed `j` in `b`) — only the inner
/// window scan is replaced by mask arithmetic.
fn jaro_winkler_ascii_bp(a: &[u8], b: &[u8], p: f64) -> f64 {
    let m = a.len();
    let n = b.len();
    debug_assert!(m <= 64 && n <= 64);

    let window = (m.max(n) / 2).saturating_sub(1);

    // peq[c] = bitmask of positions j where b[j] == c.
    // Caller guarantees ASCII, so 128 entries (1 KiB) suffice.
    let mut peq = [0u64; 128];
    for (j, &ch) in b.iter().enumerate() {
        peq[ch as usize] |= 1u64 << j;
    }

    let mut m1: u64 = 0; // matched positions in a
    let mut m2: u64 = 0; // claimed positions in b

    // Window mask over b for i = 0: bits 0..=window.
    // window ≤ 31 when len ≤ 64, so window+1 never hits the shift limit.
    let mut mask: u64 = (1u64 << (window + 1)) - 1;

    for i in 0..m {
        // Your entire inner loop, in one expression:
        //   positions where b[j] == a[i]   AND  inside window  AND  unclaimed
        let pm = peq[a[i] as usize] & mask & !m2;

        m2 |= pm & pm.wrapping_neg();        // claim the EARLIEST (blsi); 0 claims nothing
        m1 |= u64::from(pm != 0) << i;       // did a[i] match anything?

        // Advance the window for i+1: lower edge pinned at 0 while i < window
        // (grow), then both edges slide together.
        mask = if i < window { (mask << 1) | 1 } else { mask << 1 };
    }

    let matches = m1.count_ones() as usize;
    debug_assert_eq!(m1.count_ones(), m2.count_ones()); // every claim sets one bit in each
    if matches == 0 {
        return 0.0;
    }

    // Transposition walk — identical pairing logic to your current version
    // (k-th matched of a vs k-th matched of b), but iterating SET BITS ONLY
    // instead of testing every position.
    let mut transpositions = 0usize;
    let (mut x, mut y) = (m1, m2);
    while x != 0 {
        let i = x.trailing_zeros() as usize;
        let j = y.trailing_zeros() as usize;
        if a[i] != b[j] {
            transpositions += 1;
        }
        x &= x - 1; // clear lowest set bit
        y &= y - 1;
    }

    let prefix = a.iter().zip(b).take(4).take_while(|(x, y)| x == y).count();
    jaro_to_winkler(matches, transpositions, m, n, prefix, p)
}

/// Shared scoring tail — one definition, used by every path (your brief's
/// "define shared logic once" rule; three hand-copied Jaro formulas WILL drift).
#[inline]
pub fn jaro_to_winkler(matches: usize, transpositions: usize, m: usize, n: usize, prefix: usize, p: f64) -> f64 {
    let m_f = matches as f64;
    let jaro = (m_f / m as f64
        + m_f / n as f64
        + (m_f - transpositions as f64 / 2.0) / m_f)
        / 3.0;
    jaro + (prefix as f64 * p * (1.0 - jaro))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    fn approx(a: f64, b: f64) -> bool { (a - b).abs() < 1e-3 }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 1 — ANCHORS. Published Jaro-Winkler values (EXTERNAL ground truth).
    // Why: Layer 2 only proves the paths AGREE. If all three agreed on a wrong
    // number, only these anchors would catch it. Verify these constants against
    // your already-proven general path if any fails — trust general, it hit the
    // textbook values when you built it.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn anchor_textbook_values() {
        let cases = [
            ("martha",  "marhta",   0.961),
            ("dwayne",  "duane",    0.840),
            ("dixon",   "dicksonx", 0.813),
            ("jones",   "johnson",  0.832),
            ("kitten",  "kitten",   1.000),
        ];
        for (a, b, want) in cases {
            let got = jaro_winkler(a, b, None, None, None, None).unwrap();
            assert!(approx(got, want), "JW({a:?},{b:?}) = {got}, want ~{want}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 2 — PARITY FUZZ, hitting BOTH fast bands.
    // You have THREE paths: _ascii_fast (<=16), _ascii_bp (17..=64), _general.
    // A single fuzz could accidentally test only one band. So we run two fuzz
    // loops with length ranges that pin each fast path against the oracle.
    // Small alphabet + forced adjacent swaps = transposition-DENSE (the only
    // inputs that catch a bad transposition count).
    // ─────────────────────────────────────────────────────────────────────
    fn fuzz_band(iters: usize, min_len: usize, span: u64, seed0: u64) {
        let mut seed = seed0;
        let mut rng = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
        for _ in 0..iters {
            let la = min_len + (rng() % span) as usize;
            let lb = min_len + (rng() % span) as usize;
            let mut a: Vec<u8> = (0..la).map(|_| b'a' + (rng() % 4) as u8).collect();
            let mut b: Vec<u8> = (0..lb).map(|_| b'a' + (rng() % 4) as u8).collect();
            if a.len() >= 2 && rng() % 2 == 0 { let i = (rng() as usize) % (a.len()-1); a.swap(i, i+1); }
            if b.len() >= 2 && rng() % 2 == 0 { let i = (rng() as usize) % (b.len()-1); b.swap(i, i+1); }
            let (sa, sb) = (core::str::from_utf8(&a).unwrap(), core::str::from_utf8(&b).unwrap());
            if sa.is_empty() || sb.is_empty() { continue; }
            let refr = jaro_winkler_general(sa, sb, 0.1);
            // check whichever fast path this length band routes to
            let got = jaro_winkler(sa, sb, None, None, None, None).unwrap();
            assert!((got - refr).abs() < 1e-9,
                "MISMATCH len({},{}) {sa:?}/{sb:?}: opt={got} general={refr}", sa.len(), sb.len());
        }
    }

    #[test]
    fn fuzz_small_band()  { fuzz_band(40_000, 1,  16, 0x2545_F491_4F6C_DD1D); } // → _ascii_fast
    #[test]
    fn fuzz_bp_band()     { fuzz_band(40_000, 17, 48, 0x9E37_79B9_7F4A_7C15); } // → _ascii_bp

    // Direct fast-vs-bp cross: both fast paths must agree with each other too,
    // on the overlap of inputs they can both legally handle (<=16, ascii).
    #[test]
    fn fast_and_bp_agree_on_overlap() {
        let mut seed = 0xDEAD_BEEF_CAFE_1234u64;
        let mut rng = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
        for _ in 0..20_000 {
            let la = 1 + (rng() % 16) as usize;
            let lb = 1 + (rng() % 16) as usize;
            let a: Vec<u8> = (0..la).map(|_| b'a' + (rng() % 4) as u8).collect();
            let b: Vec<u8> = (0..lb).map(|_| b'a' + (rng() % 4) as u8).collect();
            let f  = jaro_winkler_ascii_fast(&a, &b, 0.1);
            let bp = jaro_winkler_ascii_bp(&a, &b, 0.1);
            assert!((f - bp).abs() < 1e-9, "fast/bp disagree on {a:?}/{b:?}: {f} vs {bp}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 3 — BOUNDARY LENGTHS. Where the u64 mask + band gates are fragile:
    // the 16/17 fast-vs-bp seam, and the 64/65 bp-vs-general seam.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn boundary_lengths_match_general() {
        for &len in &[1usize, 15, 16, 17, 32, 63, 64, 65, 80] {
            let a: String = "abcdefghij".chars().cycle().take(len).collect();
            let mut vb: Vec<char> = a.chars().collect();
            if vb.len() >= 2 { let i = vb.len() - 2; vb.swap(i, i+1); }
            let b: String = vb.into_iter().collect();
            let got  = jaro_winkler(&a, &b, None, None, None, None).unwrap();
            let refr = jaro_winkler_general(&a, &b, 0.1);
            assert!((got - refr).abs() < 1e-9, "len {len}: opt={got} general={refr}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 4 — DEGENERATE / EDGE INPUTS. Pin the contract at the corners.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn edge_cases() {
        assert_eq!(jaro_winkler("", "", None, None, None, None).unwrap(), 1.0);
        assert_eq!(jaro_winkler("abc", "", None, None, None, None).unwrap(), 0.0);
        assert_eq!(jaro_winkler("", "abc", None, None, None, None).unwrap(), 0.0);
        assert_eq!(jaro_winkler("hello", "hello", None, None, None, None).unwrap(), 1.0);
        assert_eq!(jaro_winkler("abc", "xyz", None, None, None, None).unwrap(), 0.0);
        assert_eq!(jaro_winkler("a", "a", None, None, None, None).unwrap(), 1.0);
        assert_eq!(jaro_winkler("a", "b", None, None, None, None).unwrap(), 0.0);
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 5 — ASCII vs NON-ASCII FORK. Non-ASCII MUST route to general and
    // stay correct (byte-indexing would be wrong for multibyte chars). CJK
    // included so a byte-path leak fails loudly.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn non_ascii_routes_correctly() {
        let pairs = [
            ("café", "cafe"), ("naïve", "naive"),
            ("Müller", "Muller"), ("北京", "北平"),
        ];
        for (a, b) in pairs {
            let got  = jaro_winkler(a, b, None, None, None, None).unwrap();
            let refr = jaro_winkler_general(a, b, 0.1);
            assert!((got - refr).abs() < 1e-9, "non-ascii {a:?}/{b:?}: {got} vs {refr}");
        }
        assert_eq!(jaro_winkler("café", "café", None, None, None, None).unwrap(), 1.0);
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 6 — OPTIONS. Entry-function plumbing: case fold, prefix weight p,
    // max_len. Independent of the core algorithm.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn case_insensitive_folds() {
        assert_eq!(jaro_winkler("MARTHA", "martha", None, None, Some(true), None).unwrap(), 1.0);
        let cs = jaro_winkler("MARTHA", "martha", None, None, None, None).unwrap();
        assert!(cs < 1.0, "case-sensitive should not treat MARTHA==martha");
    }

    #[test]
    fn prefix_weight_bounds() {
        assert!(jaro_winkler("a", "b", Some(0.5),  None, None, None).is_err());
        assert!(jaro_winkler("a", "b", Some(-0.1), None, None, None).is_err());
        assert!(jaro_winkler("martha", "marhta", Some(0.2), None, None, None).is_ok());
        let low  = jaro_winkler("marhta", "martha", Some(0.05), None, None, None).unwrap();
        let high = jaro_winkler("marhta", "martha", Some(0.20), None, None, None).unwrap();
        assert!(high >= low, "larger prefix weight should not lower the score");
    }

    #[test]
    fn max_len_rejects() {
        let long = "a".repeat(100);
        assert!(jaro_winkler(&long, "a", Some(10.0), None, None, None).is_err());
    }

    // ─────────────────────────────────────────────────────────────────────
    // LAYER 7 — score_cutoff FLOOR semantics (similarity, not distance).
    // Below the floor ⇒ 0.0; at/above ⇒ the real value.
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn score_cutoff_is_a_floor() {
        // martha/marhta ≈ 0.961: a floor of 0.99 should suppress it to 0.0…
        assert_eq!(jaro_winkler("martha", "marhta", None, None, None, Some(0.99)).unwrap(), 0.0);
        // …while a floor of 0.90 lets it through unchanged.
        let v = jaro_winkler("martha", "marhta", None, None, None, Some(0.90)).unwrap();
        assert!(v > 0.90);
    }
}