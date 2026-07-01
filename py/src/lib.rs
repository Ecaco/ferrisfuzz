use pyo3::prelude::*;

#[pymodule]
mod ferrisfuzz {
    use pyo3::prelude::*;

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None))]
    fn myers_distance(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool>) -> PyResult<usize> {
        ferrisfuzz_core::myers::myers_distance(str_1, str_2, max_len, case_insensitive)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None))]
    fn levenshtein_distance(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool>) -> PyResult<usize> {
        ferrisfuzz_core::levenshtein::levenshtein_distance(str_1, str_2, max_len, case_insensitive)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, p=None, max_len=None, case_insensitive=None))]
    fn jaro_winkler_distance(str_1: &str, str_2: &str, p: Option<f64>, max_len: Option<usize>, case_insensitive: Option<bool>) -> PyResult<f64> {
        ferrisfuzz_core::jaro_winkler::jaro_winkler(str_1, str_2, p, max_len, case_insensitive)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None))]
    fn damerau(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool>) -> PyResult<usize> {
        ferrisfuzz_core::damerau::damerau(str_1, str_2, max_len, case_insensitive)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }
}