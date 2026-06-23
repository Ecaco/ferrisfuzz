pub fn levenshtein_distance(str_1: &str, str_2: &str) -> usize {
    let chars_1: Vec<char> = str_1.chars().collect();
    let chars_2: Vec<char> = str_2.chars().collect();


    let m = chars_1.len();
    let n = chars_2.len();
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
    matrix[m * row_width + n]

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
                levenshtein_distance(left, right),
                expected,
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
                levenshtein_distance(left, right),
                levenshtein_distance(right, left),
                "distance should be symmetrical for {left:?} and {right:?}"
            );
        }
    }

    #[test]
    fn test_unicode() {
        assert_eq!(levenshtein_distance("é", "e"), 1);
        assert_eq!(levenshtein_distance("éa", "éb"), 1);
        assert_eq!(levenshtein_distance("猫", "犬"), 1);
        assert_eq!(levenshtein_distance("hello 🌍", "hello 🌎"), 1);
    }
}