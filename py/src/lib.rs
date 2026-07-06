use pyo3::prelude::*;

#[pymodule]
mod ferrisfuzz {
    use pyo3::prelude::*;
    use pyo3::pybacked::PyBackedStr;

    // ---------------------------------------------------------------
    // Single-pair bindings — thin, zero algorithm logic, error-mapped.
    // ---------------------------------------------------------------



    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None, score_cutoff=None))]
    fn levenshtein_bp(
        str_1: &str,
        str_2: &str,
        max_len: Option<usize>,
        case_insensitive: Option<bool>,
        score_cutoff: Option<usize>,     
    ) -> PyResult<usize> {
        ferrisfuzz_core::levenshtein_bp::levenshtein_bp(str_1, str_2, max_len, case_insensitive, score_cutoff)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }


    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None, score_cutoff=None))]
    fn damerau_bp(
        str_1: &str,
        str_2: &str,
        max_len: Option<usize>,
        case_insensitive: Option<bool>,
        score_cutoff: Option<usize>,
    ) -> PyResult<usize> {
        ferrisfuzz_core::damerau_bp::damerau_bp(str_1, str_2, max_len, case_insensitive, score_cutoff)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }


    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, p=None, max_len=None, case_insensitive=None, score_cutoff=None))]
    fn jaro_winkler_similarity(
        str_1: &str,
        str_2: &str,
        p: Option<f64>,
        max_len: Option<usize>,
        case_insensitive: Option<bool>,
        score_cutoff: Option<f64>,
    ) -> PyResult<f64> {
        ferrisfuzz_core::jaro_winkler::jaro_winkler(
            str_1, str_2, p, max_len, case_insensitive, score_cutoff,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }

    // ---------------------------------------------------------------
    // Batch bindings — zero-copy ingestion (PyBackedStr), one shared
    // `run` closure per fn, size-gated GIL detach.
    // ---------------------------------------------------------------

    const DETACH_THRESHOLD: usize = 10_000;

    #[pyfunction]
    #[pyo3(signature = (query, candidates, case_insensitive=None))]
    fn levenshtein_batch(
        py: Python<'_>,
        query: PyBackedStr,
        candidates: Vec<PyBackedStr>,
        case_insensitive: Option<bool>,
    ) -> Vec<usize> {
        let should_detach = candidates.len() >= DETACH_THRESHOLD;

        let run = move || {
            let q: &str = query.as_ref();
            let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
            ferrisfuzz_core::levenshtein_batch::levenshtein_batch(q, &refs, case_insensitive)
        };

        if should_detach { py.detach(run) } else { run() }
    }

    #[pyfunction]
    #[pyo3(signature = (query, candidates, case_insensitive=None))]
    fn damerau_batch(
        py: Python<'_>,
        query: PyBackedStr,
        candidates: Vec<PyBackedStr>,
        case_insensitive: Option<bool>,
    ) -> Vec<usize> {
        let should_detach = candidates.len() >= DETACH_THRESHOLD;

        let run = move || {
            let q: &str = query.as_ref();
            let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
            ferrisfuzz_core::damerau_batch::damerau_batch(q, &refs, case_insensitive)
        };

        if should_detach { py.detach(run) } else { run() }
    }

    #[pyfunction]
    #[pyo3(signature = (query, candidates, p=None, case_insensitive=None))]
    fn jaro_winkler_batch(
        py: Python<'_>,
        query: PyBackedStr,
        candidates: Vec<PyBackedStr>,
        p: Option<f64>,
        case_insensitive: Option<bool>,
    ) -> PyResult<Vec<f64>> {
        let should_detach = candidates.len() >= DETACH_THRESHOLD;

        let run = move || {
            let q: &str = query.as_ref();
            let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
            ferrisfuzz_core::jaro_winkler_batch::jaro_winkler_batch(q, &refs, p, case_insensitive)
        };

        let result = if should_detach { py.detach(run) } else { run() };
        result.map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }


    #[pyfunction]
    fn _ingest_owned(candidates: Vec<String>) -> usize {
        candidates.len()
    }

    #[pyfunction]
    fn _ingest_borrow(candidates: Vec<PyBackedStr>) -> usize {
        candidates.len()
    }

    #[pyfunction]
    fn _ingest_borrow_and_deref(candidates: Vec<PyBackedStr>) -> usize {
        let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
        refs.iter().map(|s| s.len()).sum()
    }
}