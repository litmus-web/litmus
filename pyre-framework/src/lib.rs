use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;


/// The matching system for routes.
///
/// This takes a given set of routes and callbacks and then compiles them
/// into a single regex system able to quickly match and scan everything.
#[pyclass]
pub struct RouterMatcher {
    /// The Python callbacks.
    routes: Vec<PyObject>,

    /// The regex set that matches any route.
    matcher: regex::RegexSet,
}

#[pymethods]
impl RouterMatcher {
    /// Creates a new matching system.
    #[new]
    pub fn new(
        routes: Vec<(String, PyObject)>,
    ) -> PyResult<Self> {
        let mut routes_new = Vec::new();
        let mut regexes = Vec::new();

        for (regex_, callback) in routes {
            routes_new.push(callback);
            regexes.push(regex_);
        }

        let matcher = match regex::RegexSet::new(regexes) {
            Ok(re) => re,
            Err(e) => return Err(PyRuntimeError::new_err(format!(
                "{:?}",
                e,
            )))
        };

        Ok(Self {
            routes: routes_new,
            matcher,
        })
    }

    /// Maybe gets a callback that matches a given path / url.
    pub fn get_callback(&self, py: Python, path: &str) -> Option<Py<PyAny>> {
        let matches = self.matcher.matches(path);

        let id = matches.iter().next()?;
        let cb = self.routes.get(id)?;
        let cloned = cb.clone_ref(py);

        return Some(cloned)
    }
}
