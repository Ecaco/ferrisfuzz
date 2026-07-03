use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct LevenshteinBatch {
    peq: [u64; 256],
    m: usize,
    mask: u64,
    fast_lane: bool,
    query: String,
    case_insensitive: Option<bool>,
}

impl LevenshteinBatch {
    pub fn new(query: &str, case_insensitive: Option<bool>) -> Self {
        let ci = case_insensitive.unwrap_or(false);
        let ascii = query.is_ascii();
        // For ASCII, byte length == char count. Otherwise count chars (fallback only).
        let m = if ascii { query.len() } else { query.chars().count() };

        let fast_lane = ascii && (1..=64).contains(&m);

        let mut peq = [0u64; 256];
        if fast_lane {
            for (i, &b) in query.as_bytes().iter().enumerate() {
                if ci {
                    peq[b.to_ascii_lowercase() as usize] |= 1u64 << i;
                    peq[b.to_ascii_uppercase() as usize] |= 1u64 << i;
                } else {
                    peq[b as usize] |= 1u64 << i;
                }
            }
        }

        let mask = if (1..=64).contains(&m) { 1u64 << (m - 1) } else { 0 };

        Self {
            peq,
            m,
            mask,
            fast_lane,
            query: query.to_string(),
            case_insensitive,
        }
    }

    #[inline]
    pub fn distance(&self, target: &str) -> usize {
        if self.m == 0 {
            return target.chars().count(); // empty query ⇒ all insertions
        }

        if self.fast_lane && target.is_ascii() {
            let mut pv: u64 = u64::MAX;
            let mut mv: u64 = 0;
            let mut score = self.m;
            let mask = self.mask;

            for &b in target.as_bytes() {
                let eq = self.peq[b as usize]; 
                let xh = ((eq & pv).wrapping_add(pv)) ^ pv;
                let x = xh | eq;
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
        } else {
            crate::levenshtein_bp::levenshtein_bp(
                &self.query,
                target,
                None,
                self.case_insensitive,
                None,
            )
            .unwrap()
        }
    }
}


pub fn levenshtein_batch(
    query: &str,
    candidates: &[&str],
    case_insensitive: Option<bool>,
) -> Vec<usize> {
    let scorer = LevenshteinBatch::new(query, case_insensitive);
    candidates.iter().map(|t| scorer.distance(t)).collect()
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::levenshtein_bp::levenshtein_bp;

    const QUERY: &str = "kitten";
    const WORDS: &[&str] = &[
        "sitting", "kitten", "mitten", "kitchen", "", "cat",
        "KITTEN", "kittens", "knitting", "café",
    ];

    #[test]
    fn batch_matches_singlepair_case_sensitive() {
        let scorer = LevenshteinBatch::new(QUERY, None);
        for &w in WORDS {
            let batch = scorer.distance(w);
            let single = levenshtein_bp(QUERY, w, None, None, None).unwrap();
            assert_eq!(batch, single, "case-sensitive mismatch on target {w:?}");
        }
    }

    #[test]
    fn batch_matches_singlepair_case_insensitive() {
        let ci = Some(true);
        let scorer = LevenshteinBatch::new(QUERY, ci);
        for &w in WORDS {
            let batch = scorer.distance(w);
            let single = levenshtein_bp(QUERY, w, None, ci, None).unwrap();
            assert_eq!(batch, single, "case-insensitive mismatch on target {w:?}");
        }
    }

    #[test]
    fn free_fn_agrees_with_singlepair() {
        let out = levenshtein_batch(QUERY, WORDS, None);
        for (i, &w) in WORDS.iter().enumerate() {
            let single = levenshtein_bp(QUERY, w, None, None, None).unwrap();
            assert_eq!(out[i], single, "free-fn mismatch on target {w:?}");
        }
    }

    #[test]
    fn case_insensitive_actually_folds() {
        let scorer = LevenshteinBatch::new("Kitten", Some(true));
        assert_eq!(scorer.distance("KITTEN"), 0); // fast lane, both-cases table
        assert_eq!(scorer.distance("kitten"), 0);
    }

    #[test]
    fn empty_query() {
        let scorer = LevenshteinBatch::new("", None);
        assert_eq!(scorer.distance("abc"), 3);
        assert_eq!(scorer.distance(""), 0);
    }

    #[test]
    fn non_ascii_query_takes_fallback_and_is_correct() {
        // Non-ASCII query ⇒ fast_lane is false ⇒ every target via the fallback.
        let scorer = LevenshteinBatch::new("café", None);
        assert_eq!(
            scorer.distance("cafe"),
            levenshtein_bp("café", "cafe", None, None, None).unwrap()
        );
        assert_eq!(scorer.distance("café"), 0);
    }

    #[test]
    fn long_query_takes_fallback() {
        // > 64 chars ⇒ multiword fallback path.
        let q = "a".repeat(70);
        let t = "a".repeat(70);
        let scorer = LevenshteinBatch::new(&q, None);
        assert_eq!(
            scorer.distance(&t),
            levenshtein_bp(&q, &t, None, None, None).unwrap()
        );
    }
}