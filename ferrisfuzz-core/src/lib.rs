#![no_std]


extern crate alloc;
pub mod levenshtein;
pub mod jaro_winkler_classic;
pub mod damerau;
pub mod common;
pub mod levenshtein_bp;
pub mod levenshtein_batch;
pub mod damerau_bp;
pub mod damerau_batch;
pub mod jaro_winkler;
pub mod jaro_winkler_batch;