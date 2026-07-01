use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;

// Bit parallel Levenshtein (Myers' algorithm) split into 2 parts. .
pub fn levenshtein_bp(str_1: &str, str_2: &str) -> usize {
    let query: Vec<char> = str_1.chars().collect();
    let m = query.len();
    let n = str_2.chars().count();

    if m == 0 { return n; }
    if n == 0 { return m; }

    // fast path: query fits in a single u64 — stack only, no heap
    if m <= 64 {
        return levenshtein_bp_small(&query, str_2, m);
    }

    levenshtein_bp_multiword(&query, str_2, m)
}

fn levenshtein_bp_small(query: &[char], target: &str, m: usize) -> usize {
    // Direct-indexed match vectors for ASCII; BTreeMap fallback for the rest.
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

    // precompute pattern match vectors (kept as BTreeMap here — the long path is rare)
    let mut pm: BTreeMap<char, Vec<u64>> = BTreeMap::new();
    for (i, &c) in query.iter().enumerate() {
        let word = i / 64;
        let bit  = i % 64;
        pm.entry(c)
          .or_insert_with(|| vec![0u64; num_words])[word] |= 1u64 << bit;
    }

    let mut pv = vec![u64::MAX; num_words];
    let mut mv = vec![0u64;     num_words];
    let mut score = m;

    let empty = vec![0u64; num_words];
    let mut xh = vec![0u64; num_words];
    let mut ph = vec![0u64; num_words];
    let mut mh = vec![0u64; num_words];

    for c in target.chars() {
        let eq = pm.get(&c).unwrap_or(&empty);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levenshtein::levenshtein_distance;

    #[test]
    fn test_basic() {
        assert_eq!(levenshtein_bp("kitten", "sitting"), 3);
    }

    #[test]
    fn test_crosscheck() {
        let pairs = [
            ("", ""),
            ("a", ""),
            ("", "a"),
            ("a", "a"),
            ("a", "b"),
            ("kitten", "sitting"),
            ("flaw", "lawn"),
            ("gumbo", "gambol"),
            ("Saturday", "Sunday"),
            ("the quick brown fox", "the slow green fox"),
        ];
        for (s1, s2) in pairs {
            let bp = levenshtein_bp(s1, s2);
            let classic = levenshtein_distance(s1, s2, None, None).unwrap();
            assert_eq!(bp, classic, "mismatch on {:?} vs {:?}", s1, s2);
        }
    }
}