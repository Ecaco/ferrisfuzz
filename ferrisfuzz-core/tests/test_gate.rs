use ferrisfuzz_core::levenshtein::levenshtein_distance_classic;
use ferrisfuzz_core::levenshtein_bp::levenshtein_bp;
use ferrisfuzz_core::levenshtein_batch::{LevenshteinBatch};
use ferrisfuzz_core::damerau::damerau_classic;
use ferrisfuzz_core::damerau_bp::damerau_bp;
use ferrisfuzz_core::damerau_batch::DamerauBatch;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const GATE_SEED_LEV: u64 = 0xFE44_15F0;
const GATE_SEED_LEV_BATCH: u64 = 0xFE44_15F0;
const GATE_SEED_OSA: u64 = 0xFE44_15F0;
const GATE_SEED_OSA_BATCH: u64 = 0xFE44_15F0;


const ALPHABET: &[u8] = b"abcde";

fn random_string(rng: &mut StdRng, max_len: usize) -> String {
    let len = rng.gen_range(0..=max_len);
    (0..len)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[test]
fn gate_levenshtein_bp_vs_classic() {
    let mut rng = StdRng::seed_from_u64(GATE_SEED_LEV);
    for i in 0..20_000 {
        let a = random_string(&mut rng, 15);
        let b = random_string(&mut rng, 15);

        let fast = levenshtein_bp(&a, &b, None, None, None).unwrap();
        let classic = levenshtein_distance_classic(&a, &b, None, None).unwrap();

        assert_eq!(
            fast, classic,
            "pair {i}: {a:?} vs {b:?} — fast={fast} classic={classic}"
        );
    }
}

#[test]
fn gate_levenshtein_batch_vs_classic() {
    let mut rng = StdRng::seed_from_u64(GATE_SEED_LEV_BATCH);
    for i in 0..5_000 {
        let a = random_string(&mut rng, 15);
        let scorer = LevenshteinBatch::new(&a, None);
        for j in 0..4 {
            let b = random_string(&mut rng, 15);
            let fast = scorer.distance(&b);
            let classic = levenshtein_distance_classic(&a, &b, None, None).unwrap();
            assert_eq!(
                fast, classic,
                "outer {i} inner {j}: query {a:?} vs target {b:?} — fast={fast} classic={classic}"
            );
        }
    }
}

#[test]
fn gate_damerau_bp_vs_classic() {
    let mut rng = StdRng::seed_from_u64(GATE_SEED_OSA);
    for i in 0..20_000 {
        let a = random_string(&mut rng, 15);
        let b = random_string(&mut rng, 15);
        let fast = damerau_bp(&a, &b, None, None, None).unwrap();
        let classic = damerau_classic(&a, &b, None, None).unwrap();
        assert_eq!(
            fast, classic,
            "pair {i}: {a:?} vs {b:?} — fast={fast} classic={classic}"
        );
    }
}

#[test]
fn gate_damerau_batch_vs_classic() {
    let mut rng = StdRng::seed_from_u64(GATE_SEED_OSA_BATCH);
    for i in 0..5_000 {
        let a = random_string(&mut rng, 15);
        let scorer = DamerauBatch::new(&a, None);
        for j in 0..4 {
            let b = random_string(&mut rng, 15);
            let fast = scorer.distance(&b);
            let classic = damerau_classic(&a, &b, None, None).unwrap();
            assert_eq!(
                fast, classic,
                "outer {i} inner {j}: query {a:?} vs target {b:?} — fast={fast} classic={classic}"
            );
        }
    }
}