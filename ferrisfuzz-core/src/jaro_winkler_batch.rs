use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::common::MatchError;
use crate::jaro_winkler::{jaro_winkler, jaro_to_winkler};

/// Compile-once Jaro-Winkler scorer: one query vs many targets.
///
/// What's hoisted to construction: `p` validation, case-folding of the query,
/// and the 1 KiB `peq` table build — the exact cost that makes single-pair
/// bit-parallel lose on short strings. Amortized here, the bp scan wins at
/// every length, so the fast lane has NO small/large split.
///
/// ORIENTATION NOTE: `peq` is built over the QUERY (single-pair builds it over
/// the second string). Jaro is symmetric so the result is identical — a claim
/// the fuzz test below verifies rather than assumes.
pub struct JaroWinklerBatch {
    peq: [u64; 128],          // ASCII fast lane only; folded if case-insensitive
    query_folded: Vec<u8>,    // folded query bytes — the scan/walk/prefix universe
    m: usize,                 // query length (bytes if ASCII, chars otherwise)
    fast_lane: bool,
    p: f64,
    query: String,            // original, for the fallback path
    case_insensitive: Option<bool>,
}

impl JaroWinklerBatch {
    pub fn new(
        query: &str,
        p: Option<f64>,
        case_insensitive: Option<bool>,
    ) -> Result<Self, MatchError> {
        // Validate ONCE here, not per target — compile-once applies to errors too.
        let p = p.unwrap_or(0.1);
        if !(0.0..=0.25).contains(&p) {
            return Err(MatchError::InvalidPrefixScale { value: p });
        }

        let ci = case_insensitive.unwrap_or(false);
        let ascii = query.is_ascii();
        let m = if ascii { query.len() } else { query.chars().count() };
        let fast_lane = ascii && (1..=64).contains(&m);

        let mut peq = [0u64; 128];
        let mut query_folded = Vec::new();
        if fast_lane {
            // NOT the Damerau both-cases trick: Jaro re-compares characters in
            // the transposition walk and prefix count, so everything must live
            // in ONE folded universe. Fold the query here; fold targets on read.
            query_folded = query
                .bytes()
                .map(|b| if ci { b.to_ascii_lowercase() } else { b })
                .collect();
            for (j, &b) in query_folded.iter().enumerate() {
                peq[b as usize] |= 1u64 << j;
            }
        }

        Ok(Self { peq, query_folded, m, fast_lane, p, query: query.to_string(), case_insensitive })
    }

    pub fn similarity(&self, target: &str) -> f64 {
        // Empty semantics identical to single-pair.
        if self.m == 0 {
            return if target.is_empty() { 1.0 } else { 0.0 };
        }
        if target.is_empty() {
            return 0.0;
        }

        if self.fast_lane && target.is_ascii() && target.len() <= 64 {
            let ci = self.case_insensitive.unwrap_or(false);
            let fold = |b: u8| if ci { b.to_ascii_lowercase() } else { b };

            let q = &self.query_folded;
            let t = target.as_bytes();
            let m = self.m;       // query length
            let n = t.len();      // target length

            let window = (m.max(n) / 2).saturating_sub(1);

            // Orientation-flipped scan: loop over TARGET chars, claim positions
            // in the QUERY. Window mask therefore ranges over query positions.
            let mut mt: u64 = 0; // matched positions in target
            let mut mq: u64 = 0; // claimed positions in query
            let mut mask: u64 = (1u64 << (window + 1)) - 1;

            for i in 0..n {
                let pm = self.peq[fold(t[i]) as usize] & mask & !mq;
                mq |= pm & pm.wrapping_neg();
                mt |= u64::from(pm != 0) << i;
                mask = if i < window { (mask << 1) | 1 } else { mask << 1 };
            }

            let matches = mt.count_ones() as usize;
            debug_assert_eq!(mt.count_ones(), mq.count_ones());
            if matches == 0 {
                return 0.0;
            }

            // Transposition walk: k-th matched target char vs k-th matched
            // query char — folded on both sides (Trap 1 lives right here).
            let mut transpositions = 0usize;
            let (mut x, mut y) = (mt, mq);
            while x != 0 {
                let i = x.trailing_zeros() as usize;
                let j = y.trailing_zeros() as usize;
                if fold(t[i]) != q[j] {
                    transpositions += 1;
                }
                x &= x - 1;
                y &= y - 1;
            }

            let mut prefix = 0usize;
            for idx in 0..m.min(n).min(4) {
                if fold(t[idx]) == q[idx] { prefix += 1; } else { break; }
            }

            // jaro_to_winkler is symmetric in (m, n); order here is irrelevant.
            jaro_to_winkler(matches, transpositions, m, n, prefix, self.p)
        } else {
            // Non-ASCII or > 64 chars: single-pair fallback, original strings.
            // p already validated, lens unbounded, cutoff not applicable → no error path.
            jaro_winkler(&self.query, target, Some(self.p), None, self.case_insensitive, None)
                .unwrap_or(0.0)
        }
    }
}

pub fn jaro_winkler_batch(
    query: &str,
    candidates: &[&str],
    p: Option<f64>,
    case_insensitive: Option<bool>,
) -> Result<Vec<f64>, MatchError> {
    let scorer = JaroWinklerBatch::new(query, p, case_insensitive)?;
    Ok(candidates.iter().map(|t| scorer.similarity(t)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jaro_winkler::jaro_winkler;

    const QUERY: &str = "martha";
    const WORDS: &[&str] = &[
        "marhta", "martha", "marta", "amrtha",           // transposition-heavy
        "sitting", "kitten", "", "m", "MARTHA", "MaRhTa",
        "café",                                           // non-ASCII → fallback
    ];

    // f64 results: epsilon comparison, never assert_eq!.
    fn assert_close(a: f64, b: f64, ctx: &str) {
        assert!((a - b).abs() < 1e-9, "{ctx}: batch={a} single={b}");
    }

    #[test]
    fn batch_matches_singlepair_case_sensitive() {
        let scorer = JaroWinklerBatch::new(QUERY, None, None).unwrap();
        for &w in WORDS {
            let single = jaro_winkler(QUERY, w, None, None, None, None).unwrap();
            assert_close(scorer.similarity(w), single, w);
        }
    }

    #[test]
    fn batch_matches_singlepair_case_insensitive() {
        let ci = Some(true);
        let scorer = JaroWinklerBatch::new(QUERY, None, ci).unwrap();
        for &w in WORDS {
            let single = jaro_winkler(QUERY, w, None, None, ci, None).unwrap();
            assert_close(scorer.similarity(w), single, w);
        }
    }

    #[test]
    fn batch_nonascii_query_falls_back() {
        let scorer = JaroWinklerBatch::new("café", None, None).unwrap();
        let single = jaro_winkler("café", "cafe", None, None, None, None).unwrap();
        assert_close(scorer.similarity("cafe"), single, "café/cafe");
    }

    #[test]
    fn batch_long_target_falls_back() {
        let scorer = JaroWinklerBatch::new(QUERY, None, None).unwrap();
        let long = "a".repeat(70);
        let single = jaro_winkler(QUERY, &long, None, None, None, None).unwrap();
        assert_close(scorer.similarity(&long), single, "long target");
    }

    #[test]
    fn invalid_p_rejected_at_construction() {
        assert!(JaroWinklerBatch::new(QUERY, Some(0.5), None).is_err());
    }

    // THE LOAD-BEARING TEST. The batch scan runs in the OPPOSITE orientation to
    // single-pair (peq over query vs peq over target). Greedy matching from
    // opposite sides *should* agree — this proves it does, on 20k adversarial
    // pairs. Small alphabet (% 5) so matches and transpositions are constant;
    // forced adjacent swaps so the transposition count actually gets exercised.
    #[test]
    fn fuzz_batch_matches_singlepair() {
        let mut seed = 0x1234_5678u64;
        let mut rng = move || {
            seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed
        };
        for _ in 0..20_000 {
            let len_a = (rng() % 20) as usize;
            let len_b = (rng() % 20) as usize;
            let mut a: String = (0..len_a).map(|_| (b'a' + (rng() % 5) as u8) as char).collect();
            let mut b: String = (0..len_b).map(|_| (b'a' + (rng() % 5) as u8) as char).collect();
            if a.len() >= 2 && rng() % 2 == 0 {
                let mut v: Vec<char> = a.chars().collect();
                let i = (rng() as usize) % (v.len() - 1);
                v.swap(i, i + 1);
                a = v.into_iter().collect();
            }
            if b.len() >= 2 && rng() % 2 == 0 {
                let mut v: Vec<char> = b.chars().collect();
                let i = (rng() as usize) % (v.len() - 1);
                v.swap(i, i + 1);
                b = v.into_iter().collect();
            }
            let scorer = JaroWinklerBatch::new(&a, None, None).unwrap();
            let single = jaro_winkler(&a, &b, None, None, None, None).unwrap();
            assert_close(scorer.similarity(&b), single, &alloc::format!("{a:?}/{b:?}"));
        }
    }
}