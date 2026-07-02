use pyo3::prelude::*;


#[pymodule]
mod ferrisfuzz {
    use pyo3::prelude::*;
    use pyo3::pybacked::PyBackedStr;

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

    #[pyfunction]
    #[pyo3(signature = (str_1, str_2, max_len=None, case_insensitive=None, score_cuttoff= None))]
    fn levenshtein_bp(str_1: &str, str_2: &str, max_len: Option<usize>, case_insensitive: Option<bool>, score_cuttoff: Option<usize>) -> PyResult<usize> {
        ferrisfuzz_core::levenshtein_bp::levenshtein_bp(str_1, str_2, max_len, case_insensitive, score_cuttoff)
        .map_err(|e|pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))
    }


        const DETACH_THRESHOLD: usize = 10_000;
    
        #[pyfunction]
        #[pyo3(signature = (query, candidates, case_insensitive=None))]
        fn levenshtein_batch(
            py: Python<'_>,
            query: PyBackedStr,
            candidates: Vec<PyBackedStr>,
            case_insensitive: Option<bool>,
        ) -> Vec<usize> {
            // Read the length BEFORE moving `candidates` into the closure — once it's
            // captured by `move`, the outer scope no longer owns it and can't inspect it.
            let should_detach = candidates.len() >= DETACH_THRESHOLD;
    
            // One shared closure so the two paths do byte-identical work and can never
            // drift apart. `move` captures query + candidates; both are PyBackedStr-
            // backed, so this is sound to run with OR without the GIL.
            let run = move || {
                let q: &str = query.as_ref();
                let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
                ferrisfuzz_core::levenshtein_batch::levenshtein_batch(q, &refs, case_insensitive)
            };
    
            if should_detach {
                // Large batch: compute dwarfs the ~104µs detach tax → release the GIL.
                py.detach(run)
            } else {
                // Small batch: hold the GIL; skipping detach saves a tax bigger than
                // the compute itself.
                run()
            }
        }
    


    #[pyfunction]
    fn _ingest_borrow_and_deref(candidates: Vec<PyBackedStr>) -> usize {
        let refs: Vec<&str> = candidates.iter().map(|c| c.as_ref()).collect();
        refs.iter().map(|s| s.len()).sum()   // touch each &str, no edit-distance
    }

    #[pyfunction]
    fn _ingest_owned(candidates: Vec<String>) -> usize {
    candidates.len()
}

    // Borrows: PyBackedStr points at the Python string's bytes, no copy.
    #[pyfunction]
    fn _ingest_borrow(candidates: Vec<PyBackedStr>) -> usize {
        candidates.len()
    }
}