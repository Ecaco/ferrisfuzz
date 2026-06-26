
#[derive(Debug, PartialEq)]
pub enum MyersErrors{
    Str1TooLarge(String),
    Str2TooLarge(String)
}

pub fn myers_distance(str_1: &str, str_2: &str, max_len: Option<usize>) -> Result<usize, MyersErrors> {
    if let Some(limit) = max_len {
        if str_1.chars().count() > limit {
            return Err(MyersErrors::Str1TooLarge(format!("String 1 length {} exceeds limit of {}", str_1.chars().count(), limit)));
            }

        if str_2.chars().count() > limit {
            return Err(MyersErrors::Str2TooLarge(format!("String 2 length {} exceeds limit of {}", str_2.chars().count(), limit)));
            }
        }

    // Conversion of strings into vectors of chars
    // eg ['f','o','r','t']
    let chars_1: Vec<char> = str_1.chars().collect();
    let chars_2: Vec<char> = str_2.chars().collect();
    let m = chars_1.len();
    let n = chars_2.len();


    // Fast Guards. If value is 0, the value will always be the size of the opposite
    if m == 0 { return Ok(n); }
    if n == 0 { return Ok(m); }

    const THRESHOLD: usize = 8;
    // stack path
        if m <= THRESHOLD && n <= THRESHOLD {
            let mut furthest = [0usize; 2 * THRESHOLD + 1];
            return Ok(myers_inner(&chars_1, &chars_2, &mut furthest, THRESHOLD));
        }
        else {
            let mut furthest = vec![0usize; 2 * (m + n) + 1];
            return Ok(myers_inner(&chars_1, &chars_2, &mut furthest, m + n));
        }



}

fn myers_inner(chars_1: &[char], chars_2: &[char], furthest: &mut [usize], offset:usize) -> usize {
    let m = chars_1.len();
    let n = chars_2.len();


    // Fast Guards. If value is 0, the value will always be the size of the opposite
    if m == 0 { return n; }
    if n == 0 { return m; }

    // Graph Setup
    let max_edits = m + n;

    // Outer loop: Progressive 'drawing' of the route through the graph 
    for k in 0..=max_edits {
        //k edits can only be on diag. step_by(2) is because each edit changes diagonal by 1, so after k edits, you can only land on diagonals
        //on the same parity as k
        for d in (-(k as isize)..=(k as isize)).step_by(2) {
            //This block: Deciding which arrow to follow into diagonal (d)
            let mut row = if d == -(k as isize) {
                furthest[(d + 1 + offset as isize) as usize]
            } else if d == k as isize {
                furthest[(d - 1 + offset as isize) as usize] + 1
            } else {
                let from_above = furthest[(d - 1 + offset as isize) as usize] + 1;
                let from_left  = furthest[(d + 1 + offset as isize) as usize];
                from_above.max(from_left)
            };
            // This is the free diagonal slide. col_signed < 0 catchest underflow bug
            let col_signed = row as isize - d;         
            if col_signed < 0 {
                furthest[(d + offset as isize) as usize] = row;
                continue;
            }
            let mut col = col_signed as usize;      
            while row < m && col < n && chars_1[row] == chars_2[col] {
                row += 1;
                col += 1;
            }
            furthest[(d + offset as isize) as usize] = row;
            if row == m && col == n {
                return k;
            }
        }
    }

    max_edits


}

#[cfg(test)]
mod tests {

use super::*;

    #[test]
    fn test_distance() {
        let str_1 = "acbd";
        let str_2 = "adcb";
        let dist = myers_distance(str_1, str_2, None);
        println!("Distance is {:?}", dist);
        assert_eq!(myers_distance(str_1, str_2, None),Ok(2))
        
    }

    #[test]
    fn test_ratcat() {
        assert_eq!(myers_distance("short", "fort", None), Ok(3))
    }

    #[test]
    fn test_max_str_1() {
        assert_eq!(myers_distance("Bigger", "Big", Some(3)), Err(MyersErrors::Str1TooLarge("String 1 length 6 exceeds limit of 3".to_string())));
    }

        #[test]
    fn test_max_str_2() {
        assert_eq!(myers_distance("Big", "Bigger", Some(3)), Err(MyersErrors::Str2TooLarge("String 2 length 6 exceeds limit of 3".to_string())));
    }
}
