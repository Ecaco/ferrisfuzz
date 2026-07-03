use crate::alloc::string::ToString;
use alloc::string::String;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;


#[derive(Debug, PartialEq)]
pub enum LevenshteinError {
    InputTooLong(String)
}

pub fn levenshtein_distance_classic(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool>) -> Result<usize, LevenshteinError> {
    let limit = max_len.unwrap_or(10_000);
    let case_insensitive= case_insensitive.unwrap_or(false);


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


    let chars_1: Vec<char> = str_1.chars().collect();
    let chars_2: Vec<char> = str_2.chars().collect();

    let m = chars_1.len();  
    let n = chars_2.len();

    if m == 0 && n == 0 {
        return Ok(0);
    }
    if m == 0 {
        return Ok(n);
    }
    if n == 0 {
        return Ok(m);
    }

    
    if m > limit {
    return Err(LevenshteinError::InputTooLong(format!("str_1 has an input value of {}: character limit is {}", m, limit)));
    } else if n > limit {
    return Err(LevenshteinError::InputTooLong(format!("str_2 has an input value of {}: Character limit is {}", n, limit)))
    }
    
    let row_width = n + 1;
    
    let mut matrix = vec![0; (m + 1) * (n + 1)];

    for col in 0..=n {
        let target_index = col;
        matrix[target_index] = col
    }

    for row in 0..=m{
        matrix[row * row_width] = row;
    }

    for row in 1..=m{
        for col in 1..=n {
        let current_idx = row * row_width + col;
        let top_idx     = (row - 1) * row_width + col;
        let left_idx    = row * row_width + (col - 1);
        let diagonal_idx = (row - 1) * row_width + (col - 1);

        if chars_1[row - 1] == chars_2[col - 1]  {
            matrix[current_idx] = matrix[diagonal_idx]
            }
        else {
            let a = matrix[top_idx];
            let b = matrix[left_idx];
            let c = matrix[diagonal_idx];

            let min_cost = a.min(b).min(c);
            matrix[current_idx] = 1 + min_cost
        }
        }
    }
    Ok(matrix[m * row_width + n])

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_distances() {
        let cases = [
            ("", "", 0),
            ("a", "", 1),
            ("", "a", 1),
            ("a", "a", 0),
            ("a", "b", 1),
            ("abc", "abc", 0),
            ("abc", "ab", 1),
            ("ab", "abc", 1),
            ("kitten", "sitting", 3),
            ("flaw", "lawn", 2),
            ("gumbo", "gambol", 2),
            ("book", "back", 2),
            ("Saturday", "Sunday", 3),
        ];

        for (left, right, expected) in cases {
            assert_eq!(
                levenshtein_distance_classic(left, right, None, None),
                Ok(expected),
                "failed for {left:?} vs {right:?}"
            );
        }
    }

    #[test]
    fn test_symmetry() {
        let pairs = [
            ("kitten", "sitting"),
            ("flaw", "lawn"),
            ("abc", "xyz"),
            ("rust", "trust"),
        ];

        for (left, right) in pairs {
            assert_eq!(
                levenshtein_distance_classic(left, right, None, None),
                levenshtein_distance_classic(right, left, None, None),
                "distance should be symmetrical for {left:?} and {right:?}"
            );
        }
    }

    #[test]
    fn test_unicode() {
        assert_eq!(levenshtein_distance_classic("é", "e", None, None), Ok(1));
        assert_eq!(levenshtein_distance_classic("éa", "éb", None, None), Ok(1));
        assert_eq!(levenshtein_distance_classic("猫", "犬", None, None), Ok(1));
        assert_eq!(levenshtein_distance_classic("hello 🌍", "hello 🌎", None, None), Ok(1));
    }

    #[test]
    fn crosstest_myers() {
        assert_eq!(levenshtein_distance_classic("acbd", "adcb", None, None), Ok(2));
    }

    #[test]
    fn test_limits() {
        assert_eq!(levenshtein_distance_classic("acbd", "adcb", Some(3), None), Err(LevenshteinError::InputTooLong("str_1 has an input value of 4: character limit is 3".to_string())))
    }
}