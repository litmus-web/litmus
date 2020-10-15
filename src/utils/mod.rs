use pyo3::prelude::*;
use regex::Regex;


pub fn url_test_regex(text: &str, re: Regex) -> bool {
    for _ in 0..1000 {
        re.is_match(text);
    }
    re.is_match(text)
}

pub fn make_regex_from_vec(patterns: Vec<(&str, PyObject)>) -> Vec<(Regex, PyObject)> {
    let mut output = Vec::new();

    for pattern in patterns {
        let re = Regex::new(pattern.0).unwrap();
        output.push((re, pattern.1));
    }

    output
}