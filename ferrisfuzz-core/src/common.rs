//! Shared, opt-in preamble helpers. The guiding rule: everything here is skipped
//! (or borrows, or is a predicted-not-taken branch) when the caller doesn't ask
//! for it, so the default fast path pays for nothing it doesn't use.

use alloc::borrow::Cow;
use alloc::string::String;

#[derive(Debug, PartialEq)]
pub enum MatchError {
    /// One input exceeded the configured character cap.
    InputTooLong { which: &'static str, len: usize, limit: usize },
    InvalidPrefixScale {value: f64},
}

impl MatchError {
    /// Human-readable message (handy when mapping to a Python exception).
    pub fn message(&self) -> String {
        match self {
            MatchError::InputTooLong { which, len, limit } => {
                alloc::format!("{which} has {len} characters; the limit is {limit}")
            }
            MatchError::InvalidPrefixScale { value } => {
                alloc::format!("Invalid prefix value: {value}")
            }
        }
    }
}

/// Opt-in case folding, done ONCE here — never per character in a loop.
///
/// Case-sensitive (the default) returns `Cow::Borrowed`: zero allocation, points
/// straight at the caller's string. Only case-insensitive allocates the single
/// lowercased copy. This mirrors rapidfuzz's "no preprocessing unless asked".
#[inline]
pub fn normalize(s: &str, case_insensitive: Option<bool>) -> Cow<'_, str> {
    if case_insensitive.unwrap_or(false) {
        Cow::Owned(s.to_lowercase())
    } else {
        Cow::Borrowed(s)
    }
}

/// Opt-in length cap. A no-op when `max_len` is `None`.
#[inline]
pub fn check_len(len: usize, max_len: Option<usize>, which: &'static str) -> Result<(), MatchError> {
    if let Some(limit) = max_len {
        if len > limit {
            return Err(MatchError::InputTooLong { which, len, limit });
        }
    }
    Ok(())
}

/// Apply score-cutoff semantics to a final distance: if it exceeds the cutoff,
/// report `cutoff + 1` (rapidfuzz's convention) rather than the exact value.
/// A no-op when `score_cutoff` is `None`.
#[inline]
pub fn apply_cutoff(score: usize, score_cutoff: Option<usize>) -> usize {
    match score_cutoff {
        Some(k) if score > k => k + 1,
        _ => score,
    }
}