use pyo3::prelude::*;

#[pymodule]
mod ferrisfuzz {
    use pyo3::prelude::*;

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None))]
    fn myers_distance(str_1: &str, str_2: &str, max_len: Option<usize>) -> PyResult<usize> {
        ferrisfuzz_core::myers::myers_distance(str_1, str_2, max_len)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2))]
    fn levenshtein_distance(str_1: &str, str_2: &str) -> PyResult<usize> {
        Ok(ferrisfuzz_core::levenshtein::levenshtein_distance(str_1, str_2))
    }
}