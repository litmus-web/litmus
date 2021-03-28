use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

macro_rules! conv_err {
    ( $e:expr ) => ( $e.map_err(|e| PyRuntimeError::new_err(format!("{}", e))) )
}


/// The matching system for routes.
///
/// This takes a given set of routes and callbacks and then compiles them
/// into a single regex system able to quickly match and scan everything.
#[pyclass]
pub struct RouterMatcher {
    /// The Python callbacks.
    routes: Vec<(regex::Regex, PyObject)>,

    /// The regex set that matches any route.
    matcher: regex::RegexSet,
}

#[pymethods]
impl RouterMatcher {
    /// Creates a new matching system.
    #[new]
    pub fn new(
        routes: Vec<(&str, PyObject)>,
    ) -> PyResult<Self> {
        let mut routes_new = Vec::new();
        let mut regexes = Vec::new();

        for (regex_, callback) in routes {
            let re = conv_err!(regex::Regex::new(regex_))?;
            routes_new.push((re, callback));
            regexes.push(regex_);
        }

        let matcher = conv_err!(regex::RegexSet::new(regexes))?;

        Ok(Self {
            routes: routes_new,
            matcher,
        })
    }

    /// Maybe gets a callback that matches a given path / url.
    pub fn get_callback(&self, py: Python, path: &str) -> Option<(Py<PyAny>, Vec<(String, String)>)> {
        let matches = self.matcher.matches(path);

        let id = matches.iter().next()?;
        let (re, cb) = self.routes.get(id)?;

        let names: Vec<Option<&str>> = re.capture_names().collect();
        let cap = re.captures(path)?;

        let mut out: Vec<(String, String)> = Vec::new();
        for name in names {
            if let Some(name) = name {
                if let Some(match_) = cap.name(name) {
                    let val = match_.as_str().into();
                    out.push((name.into(), val))
                }
            }
        }

        let cloned = cb.clone_ref(py);

        Some((cloned, out))
    }
}
