pub fn levenshtein_distance(str_1: String, str_2: String) -> usize {
    let m = str_1.len();
    let n = str_2.len();
    let row_width = n + 1;
    
    let mut matrix = vec![0; (m + 1) * (n + 1)];

    for col in 0..=n {
        let target_index = col;
        matrix[target_index] = col
    }

    for row in 0..=m{
        let target_row = row;
        matrix[row * row_width] = row;
    }

    for row in 1..=m{
        for col in 1..=n {
        let current_idx = row * row_width + col;
        let top_idx     = (row - 1) * row_width + col;
        let left_idx    = row * row_width + (col - 1);
        let diagonal_idx = (row - 1) * row_width + (col - 1);

        if str_1.as_bytes()[row - 1] == str_2.as_bytes()[col - 1] {
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
    fn it_works() {
        let string_1 = "Kitten".to_string();
        let string_2 = "Mitten".to_string();
        assert_eq!(levenshtein_distance(string_1, string_2), 1)
    }
}