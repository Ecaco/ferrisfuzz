use crate::alloc::string::ToString;
use alloc::string::String;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, PartialEq)]
pub enum DamerauError {
    InputTooLong(String)
}

pub fn damerau_classic(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool> ) -> Result<usize, DamerauError> {
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

    let m = str_1.chars().count();
    let n = str_2.chars().count();


    if m == 0 && n == 0 {
        return Ok(0);
    }
    if m == 0 {
        return Ok(n);
    }
    if n == 0 {
        return Ok(m);
    }

    let limit = max_len.unwrap_or(10_000);

    if m > limit {
    return Err(DamerauError::InputTooLong(format!("str_1 has an input value of {}: character limit is {}", m, limit)));
    } else if n > limit {
    return Err(DamerauError::InputTooLong(format!("str_2 has an input value of {}: Character limit is {}", n, limit)))
    }


    let row_width = n + 1;
    
    let mut matrix = vec![0; (m + 1) * (n + 1)];

    for col in 0..=n {
        let target_index = col;
        matrix[target_index] = col
    }

    for row in 0..=m{
        matrix[row * row_width] = row;
    };

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

            let mut min_cost = a.min(b).min(c);

            // damerau_classic transposition check
            if row > 1 && col > 1
            && chars_1[row - 1] == chars_2[col - 2] 
            && chars_1[row - 2] == chars_2[col - 1]
            {
                let transpose_idx = (row - 2) * row_width + (col - 2);
                let transpose_cost = matrix[transpose_idx];
                min_cost = min_cost.min(transpose_cost)
            }

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
    fn test_ac_ca() {
        assert_eq!(damerau_classic("CA", "AC", None, None), Ok(1) );
    }

    #[test]
    fn test_complex() {
        assert_eq!(damerau_classic("ABCDEFG", "BACDFEG", None, None), Ok(2))
    }

    #[test] 
    fn test_string_length_guard() {
        let mut long_string = String::from("A");
        for i32 in 0..10_005 {
            long_string.push('A') 
            }

        assert_eq!(damerau_classic(&long_string, "test", None, None), Err(DamerauError::InputTooLong("str_1 has an input value of 10006: character limit is 10000".to_string())))
    }

    #[test]
    fn test_empty_strings() {
        assert_eq!(damerau_classic("", "", None, None), Ok(0) );
    }

    #[test]
    fn test_one_empty() {
        assert_eq!(damerau_classic("Test", "", None, None), Ok(4) );
    }

    #[test]
    fn test_theory() {
        assert_eq!(damerau_classic("a", "abc", None, None), Ok(2))
    }
}