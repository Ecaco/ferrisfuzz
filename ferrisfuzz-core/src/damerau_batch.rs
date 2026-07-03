use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct DamerauBatch {
    peq: [u64; 256],
    m: usize,
    mask: u64,
    fast_lane: bool,
    query: String,
    case_insensitive: Option<bool>,
}

impl DamerauBatch {
    pub fn new(query: &str, case_insensitive: Option<bool>) -> Self {
        let ci = case_insensitive.unwrap_or(false);
        let ascii = query.is_ascii();
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

    pub fn distance(&self, target: &str) -> usize {
        if self.m == 0 {
            return target.chars().count();
        }

        if self.fast_lane && target.is_ascii() {
            let mut pv: u64 = u64::MAX;
            let mut mv: u64 = 0;
            let mut score = self.m;
            let mask = self.mask;

            let mut x: u64 = 0;
            let mut pm_old: u64 = 0;

            for &b in target.as_bytes() {
                let eq = self.peq[b as usize];


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
        } else {
            crate::damerau_bp::damerau_bp(
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

pub fn damerau_batch(
    query: &str,
    candidates: &[&str],
    case_insensitive: Option<bool>,
) -> Vec<usize> {
    let scorer = DamerauBatch::new(query, case_insensitive);
    candidates.iter().map(|t| scorer.distance(t)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damerau_bp::damerau_bp;

    const QUERY: &str = "kitten";
    const WORDS: &[&str] = &[
        "sitting", "kitten", "mitten", "kitchen", "", "cat",
        "KITTEN", "kittens", "knitting", "teh", "hte",   // transposition cases
        "café",                                          // non-ASCII → fallback
    ];

    #[test]
    fn batch_matches_singlepair_case_sensitive() {
        let scorer = DamerauBatch::new(QUERY, None);
        for &w in WORDS {
            let batch = scorer.distance(w);
            let single = damerau_bp(QUERY, w, None, None, None).unwrap();
            assert_eq!(batch, single, "cs mismatch on {w:?}");
        }
    }

    #[test]
    fn batch_matches_singlepair_case_insensitive() {
        let ci = Some(true);
        let scorer = DamerauBatch::new(QUERY, ci);
        for &w in WORDS {
            let batch = scorer.distance(w);
            let single = damerau_bp(QUERY, w, None, ci, None).unwrap();
            assert_eq!(batch, single, "ci mismatch on {w:?}");
        }
    }

    #[test]
    fn batch_transposition_and_nonascii() {
        // Explicit transposition + fallback coverage on the batch path.
        let scorer = DamerauBatch::new("teh", None);
        assert_eq!(scorer.distance("the"), damerau_bp("teh", "the", None, None, None).unwrap());

        let s2 = DamerauBatch::new("café", None); // non-ASCII query → fallback for all
        assert_eq!(s2.distance("cafe"), damerau_bp("café", "cafe", None, None, None).unwrap());
    }

    #[test]
    fn batch_long_multiword() {
        let q = "a".repeat(70);
        let scorer = DamerauBatch::new(&q, None);
        let t = "a".repeat(70);
        assert_eq!(scorer.distance(&t), damerau_bp(&q, &t, None, None, None).unwrap());
    }
}