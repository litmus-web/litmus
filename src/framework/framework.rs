use pyo3::prelude::*;
use regex::Regex;
use once_cell::sync::OnceCell;

use crate::utils;


static URL_REGEX: OnceCell<Vec<(Regex, PyObject)>> = OnceCell::new();


fn framework_init(
    py: Python,                             // General pass though for future use
    regex_patterns: Vec<(&str, PyObject)>,  // Url routing and their relevant callbacks
) -> PyResult<()> {
    // Set-up regex on the global scale to help efficiency.
    let _: &Vec<(Regex, PyObject)> = URL_REGEX.get_or_init(|| {
        utils::make_regex_from_vec(regex_patterns)
    });

    Ok(())
}


#[pyclass]
struct RustProtocol {

}

#[pymethods]
impl RustProtocol {
    #[new]
    fn new(_py: Python, ) -> PyResult<Self> {
        Ok(RustProtocol{})
    }


}