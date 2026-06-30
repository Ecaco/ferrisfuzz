
#[derive(Debug, PartialEq)]
pub enum JaroWinklerError {
    InvalidPrefixScale(String),
    InputTooLong(String),
}

pub fn jaro_winkler(str_1: &str, str_2: &str, p: Option<f64>, max_len: Option<usize>, case_insensitive: Option<bool> ) -> Result<f64, JaroWinklerError> {
    // Guards
    let limit = max_len.unwrap_or(100_000); // Linear so can be generous
    if str_1.trim().is_empty() && str_2.trim().is_empty() {
        return Ok(1.0);
    };
    
    if str_1.trim().is_empty() || str_2.trim().is_empty() {
        return Ok(0.0);
    };

    let p = p.unwrap_or(0.1);
    // Anything above 0.25 will can get scores above 1, which breaks a similarity scale, anything negative would penalise shared prefixes.
    if p < 0.0 || p > 0.25 {
        return Err(JaroWinklerError::InvalidPrefixScale(format!("{} is invalid: must be between 0.0 and 0.25", p)));
    };

    let case_insensitive = case_insensitive.unwrap_or(false);

    // Case insensitive param settings
    let str_1 = if case_insensitive {
        str_1.to_lowercase()
    } else {
        str_1.to_string()
    };
    let str_2 = if case_insensitive {
        str_2.to_lowercase()
    } else {
        str_2.to_string()
    };

    let m = str_1.chars().count();
    let n = str_2.chars().count();

    if m > limit {
       return Err(JaroWinklerError::InputTooLong(format!("str_1 has an input value of {}: character limit is {}", m, limit)));
    } else if n > limit {
        return Err(JaroWinklerError::InputTooLong(format!("str_2 has an input value of {}: Character limit is {}", n, limit)))
    }

    // Saturating sub: Stop it going below 0 (can't have a negative window)
    let window = (m.max(n) / 2).saturating_sub(1);

    // initialisation of array of chars, boolean array for matches and match counter
    let chars_1: Vec<char> = str_1.chars().collect();
    let chars_2: Vec<char> = str_2.chars().collect();
    let mut str1_matches = vec![false; m];
    let mut str2_matches = vec![false; n];
    let mut matches = 0usize;

    // Nested loop. for each char index from 0 to m create the window bounds
    // Then loop through that window to find matches
    for i in 0..m {
        // calculate the window bounds for this position
        let start = i.saturating_sub(window);
        let end = (i + window + 1).min(n);
        
        for j in start..end {
            // skip if str_2[j] already matched
            if str2_matches[j] {
                continue;
            }
            // skip if characters don't match
            else if chars_1[i] != chars_2[j] {
                continue;
            }
            // otherwise: mark both as matched, increment matches, break
            else {
                str1_matches[i] = true;  // note: i not j
                str2_matches[j] = true;
                matches += 1;
                break;
            }
        }
    }


    let mut transpositions = 0usize;
    let mut k = 0usize;

    // k walks through matched positions in str_2 in order
    // if the matched char in str_1 differs from the corresponding matched char in str_2
    // they're in a different order and are that's a transposition
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

    //casting for end result
    let m = m as f64;
    let n = n as f64;
    let matches = matches as f64;
    let transpositions = transpositions as f64;

    // escape hatch: if there are no matches, just return without running calc
    if matches == 0.0 {
    return Ok(0.0);
}
    // jaro part: 
    let jaro = (matches/m + matches/n + (matches - transpositions/2.0) /matches) / 3.0;

    // winkler part: 
    let prefix = chars_1.iter()
    .zip(chars_2.iter())
    .take(4)
    .take_while(|(a, b)| a == b)
    .count() as f64;

    // combined
    let jaro_winkler = jaro + (prefix * p * (1.0 - jaro));

    Ok(jaro_winkler)
    
}

#[cfg(test)]
mod tests {

use super::*;

    #[test] 
    fn test_jaro_martha() {
        let result = jaro_winkler("MARTHA", "MARHTA", Some(0.0), None, None).unwrap();
        assert!((result - 0.944).abs() < 0.001, "got {}", result);
        
    }

    #[test]
    fn test_jaro_winkler_martha() {
        let result = jaro_winkler("MARTHA", "MARHTA", None, None, None).unwrap();
        assert!((result - 0.961).abs() < 0.001, "got {}", result);
    }

    #[test]
    fn test_empty_string_2() {
        let result = jaro_winkler("MARTHA", "",Some(0.0), None, None);
        assert_eq!(result, Ok(0.0))
    }

    #[test]
    fn test_empty_string_1() {
        let result = jaro_winkler("", "MARTHA",Some(0.0), None, None);
        assert_eq!(result, Ok(0.0))
    }

    #[test]
    fn test_empty_strings() {
        let result = jaro_winkler("", "", Some(0.0), None, None);
        assert_eq!(result, Ok(1.0))
    }

    #[test]
    fn test_upper_p() {
        let result = jaro_winkler("MARTHA", "MARTER", Some(0.5), None, None);
        assert_eq!(result, Err(JaroWinklerError::InvalidPrefixScale("0.5 is invalid: must be between 0.0 and 0.25".to_string())))
    }

    #[test]
    fn test_lower_p() {
        let result = jaro_winkler("MARTHA", "MARTER", Some(-0.5), None, None);
        assert_eq!(result, Err(JaroWinklerError::InvalidPrefixScale("-0.5 is invalid: must be between 0.0 and 0.25".to_string())))
    }

    #[test]
    fn test_unicode_jw_match() {
        let result = jaro_winkler("😂", "😂", Some(0.0), None, None);
        assert_eq!(result, Ok(1.0))
    }
        
    #[test]
    fn test_unicode_jw_dif() {
        let result = jaro_winkler("🙂", "😂", Some(0.0), None, None);
        assert_eq!(result, Ok(0.0))
    }

    #[test]
    fn test_crate_trace_case() {
        let result = jaro_winkler("CRATE", "trace", Some(0.0), None , Some(true)).unwrap();
        assert!((result - 0.733).abs() < 0.001, "got {}", result);
    }

    #[test]
    fn test_char_limit() {
        let result = jaro_winkler("MARTHA", "MARHTA", Some(0.0), Some(5), None);
        assert_eq!(result, Err(JaroWinklerError::InputTooLong("str_1 has an input value of 6: character limit is 5".to_string())))
    }

}
