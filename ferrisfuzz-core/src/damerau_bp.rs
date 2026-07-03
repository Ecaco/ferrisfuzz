use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::common::{MatchError, check_len, apply_cutoff, normalize};
use alloc::vec;
use alloc::string::String;


pub fn damerau_bp(
    str_1: &str, 
    str_2: &str, 
    max_len: Option<usize>, 
    case_insensitive: Option<bool>,
    score_cutoff: Option<usize> ) 
    -> Result<usize, MatchError> {

    let s1 = normalize(str_1, case_insensitive);
    let s2 = normalize(str_2, case_insensitive);

    let ascii_only = s1.is_ascii() && s2.is_ascii();

    // On the ASCII path, byte length == char length — so m/n need no decode
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
        damerau_bp_small_bytes(s1.as_bytes(), s2.as_bytes(), m)
    } else {
        let query: Vec<char> = s1.chars().collect();
        if m <= 64 {
            damerau_bp_small(&query, s2.as_ref(), m)
        } else {
            damerau_bp_multiword(&query, s2.as_ref(), m)
        }
    };

    Ok(apply_cutoff(score, score_cutoff))
}


fn damerau_bp_small(query: &[char], target: &str, m: usize) -> usize {
    // --- match table: IDENTICAL to levenshtein_bp_small ---
    // Direct-indexed vectors for chars < 256; BTreeMap fallback beyond.
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

    let mut x: u64 = 0;
    let mut pm_old: u64 = 0;

    for c in target.chars() {
        let cp = c as u32;
        let eq = if cp < 256 {
            ascii[cp as usize]
        } else {
            wide.get(&c).copied().unwrap_or(0)
        };

        let tr = (((!x) & eq) << 1) & pm_old;

        // Myers core (same recurrence as Levenshtein), then fold in the transposition.
        let xh = ((eq & pv).wrapping_add(pv)) ^ pv;
        x = xh | eq | mv;   
        x |= tr;            

        let ph = mv | !(x | pv);
        let mh = pv & x;

        if ph & mask != 0 { score += 1; }
        if mh & mask != 0 { score -= 1; }

        let ph_shift = (ph << 1) | 1;
        let mh_shift = mh << 1;
        pv = mh_shift | !(x | ph_shift);
        mv = ph_shift & x;

        pm_old = eq;       
    }

    score
}

fn damerau_bp_multiword(query: &[char], target: &str, m: usize) -> usize {
    let num_words = (m + 63) / 64;
    let last_mask = 1u64 << ((m - 1) % 64);

    // Match table: identical to levenshtein_bp_multiword
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

    //    DAMERAU state as VECTORS:
    //   `x`      — the diagonal, PERSISTED across target chars (Levenshtein threw it
    //              away each step; Damerau needs the previous step's value).
    //   `pm_old` — previous target char's match vector.
    //   `tr`     — scratch for the transposition term.
    let mut x  = vec![0u64; num_words];
    let mut pm_old = vec![0u64; num_words];
    let mut tr = vec![0u64; num_words];

    for c in target.chars() {
        let cp = c as u32;
        let eq: &[u64] = if cp < 256 {
            let base = cp as usize * num_words;
            &ascii[base..base + num_words]
        } else {
            wide.get(&c).map(|v| v.as_slice()).unwrap_or(&empty)
        };

        let mut carry_tr = 0u64;
        for i in 0..num_words {
            let t_i      = (!x[i]) & eq[i];       
            let shifted  = (t_i << 1) | carry_tr; 
            carry_tr     = t_i >> 63;                  
            tr[i]        = shifted & pm_old[i];        
        }

        // Myers horizontal add with carry (identical to Levenshtein).
        let mut carry = 0u64;
        for i in 0..num_words {
            let x_i = eq[i] & pv[i];
            let (s1, c1) = x_i.overflowing_add(pv[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            carry = (c1 || c2) as u64;
            xh[i] = s2 ^ pv[i];
        }

        // Assemble this step's diagonal, then fold in the transposition.
        for i in 0..num_words {
            x[i] = xh[i] | eq[i] | mv[i] | tr[i];  
            ph[i] = mv[i] | !(x[i] | pv[i]);
            mh[i] = pv[i] & x[i];
        }

        let last = num_words - 1;
        if ph[last] & last_mask != 0 { score += 1; }
        if mh[last] & last_mask != 0 { score = score.saturating_sub(1); }

        let mut carry_ph = 1u64;
        let mut carry_mh = 0u64;
        for i in 0..num_words {
            let ph_shift = (ph[i] << 1) | carry_ph;
            let mh_shift = (mh[i] << 1) | carry_mh;
            carry_ph     = ph[i] >> 63;
            carry_mh     = mh[i] >> 63;
            pv[i] = mh_shift | !(x[i] | ph_shift);
            mv[i] = ph_shift & x[i];
        }


        pm_old.copy_from_slice(eq);
    }

    score
}

fn damerau_bp_small_bytes(query: &[u8], target: &[u8], m: usize) -> usize {
    // Match table: pure byte-indexed array. No BTreeMap, no char decode — every
    // byte is < 256 by definition. This is the ONLY difference from damerau_bp_small;
    // the transposition machinery below is identical (it works on bit-vectors, not
    // on how the match vector was looked up).
    let mut peq = [0u64; 256];
    for (i, &b) in query.iter().enumerate() {
        peq[b as usize] |= 1u64 << i;
    }

    let mut pv: u64 = u64::MAX;
    let mut mv: u64 = 0;
    let mut score = m;
    let mask = 1u64 << (m - 1);

    // Same Damerau state as the char path: persistent `x`, carried `pm_old`.
    let mut x: u64 = 0;
    let mut pm_old: u64 = 0;

    for &b in target.iter() {
        let eq = peq[b as usize]; // single load, no decode, no branch

        let tr = (((!x) & eq) << 1) & pm_old;

        let xh = ((eq & pv).wrapping_add(pv)) ^ pv;
        x = xh | eq | mv;
        x |= tr;

        let ph = mv | !(x | pv);
        let mh = pv & x;

        if ph & mask != 0 { score += 1; }
        if mh & mask != 0 { score -= 1; }

        let ph_shift = (ph << 1) | 1;
        let mh_shift = mh << 1;
        pv = mh_shift | !(x | ph_shift);
        mv = ph_shift & x;

        pm_old = eq;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damerau::damerau;

    #[test]
    fn bp_matches_classic_osa_transpositions() {
        let pairs = [
            ("ca", "abc"), ("ac", "cba"), ("teh", "the"),
            ("abcd", "acbd"), ("abcdef", "abcfed"),
            ("a cat", "a abct"), ("kitten", "sitting"),
            ("", "abc"), ("abc", ""), ("aa", "aa"),
        ];
    for (a, b) in pairs {
        let bp = damerau_bp(a, b, None, None, None).unwrap();   // entry fn, not _small
        let classic = damerau(a, b, None, None).unwrap();
        assert_eq!(bp, classic, "mismatch on {a:?}/{b:?}: bp={bp} classic={classic}");
    }
    }

    #[test]
    fn bp_multiword_matches_classic() {
        // Must exceed 64 chars to hit the multiword path. Include transpositions,
        // and critically one NEAR position 64 to stress the cross-word carry.
        let a = "the quick brown fox jumps over the lazy dog then runs home quickly xy";
        let b = "the quick brown fox jumps over the lazy dog then runs home quickly yx"; // xy->yx swap at the end (pos 65)
        assert_eq!(
            damerau_bp(a, b, None, None, None).unwrap(),
            damerau(a, b, None, None).unwrap()
        );

        // A few more: long identical, long with mid transposition, long with edits.
        let base: String = "abcdefghij".repeat(8); // 80 chars
        let mut swapped: Vec<char> = base.chars().collect();
        swapped.swap(63, 64); // transposition EXACTLY across the word boundary — the danger spot
        let swapped: String = swapped.into_iter().collect();
        assert_eq!(
            damerau_bp(&base, &swapped, None, None, None).unwrap(),
            damerau(&base, &swapped, None, None).unwrap()
        );
    }
}