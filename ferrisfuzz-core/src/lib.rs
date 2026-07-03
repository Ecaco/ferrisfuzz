#![no_std]

//! Classic O(mn) dynamic-programming reference. Not part of the public API —
//! retained as the crosscheck oracle that proves the bit-parallel path correct.

extern crate alloc;
mod levenshtein;
mod jaro_winkler;
mod damerau;
pub mod common;
pub mod levenshtein_bp;
pub mod levenshtein_batch;
pub mod damerau_bp;
pub mod damerau_batch;
pub mod jaro_winkler_opt;
pub mod jaro_winkler_batch;