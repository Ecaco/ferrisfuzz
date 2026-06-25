pub fn myers_distance(str_1: &str, str_2: &str) -> usize {
    let chars_1: Vec<char> = str_1.chars().collect();
    let chars_2: Vec<char> = str_2.chars().collect();
    let m = chars_1.len();
    let n = chars_2.len();

    if m == 0 { return n; }
    if n == 0 { return m; }

    let max_edits = m + n;
    let offset = max_edits;
    let mut furthest = vec![0usize; 2 * max_edits + 1];

    for k in 0..=max_edits {
        for d in (-(k as isize)..=(k as isize)).step_by(2) {
            let mut row = if d == -(k as isize) {
                furthest[(d + 1 + offset as isize) as usize]
            } else if d == k as isize {
                furthest[(d - 1 + offset as isize) as usize] + 1
            } else {
                let from_above = furthest[(d - 1 + offset as isize) as usize] + 1;
                let from_left  = furthest[(d + 1 + offset as isize) as usize];
                from_above.max(from_left)
            };
            let col_signed = row as isize - d;          // was: row + d
            if col_signed < 0 {
                furthest[(d + offset as isize) as usize] = row;
                continue;
            }
            let mut col = col_signed as usize;          // was: (row + d) as usize
            while row < m && col < n && chars_1[row] == chars_2[col] {
                // print BEFORE advancing, so indices stay in bounds
                println!("    matched chars_1[{}]={} chars_2[{}]={}", row, chars_1[row], col, chars_2[col]);
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
        let dist = myers_distance(str_1, str_2);
        println!("Distance is {:?}", dist);
        assert_eq!(myers_distance(str_1, str_2),2)
        
    }

    #[test]
fn test_unicode() {
    let cases = [
        ("é", "e", 2),
        ("éa", "éb", 2),
        ("猫", "犬", 2),
        ("hello 🌍", "hello 🌎", 2),
    ];
    for (a, b, expected) in cases {
        let got = myers_distance(a, b);
        println!("{:?} vs {:?} = {} (expected {})", a, b, got, expected);
    }
}
    #[test]
    fn test_ratcat() {
        assert_eq!(myers_distance("short", "fort"), 3)
    }
}
