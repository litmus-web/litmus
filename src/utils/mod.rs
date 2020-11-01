use std::collections::HashMap;
use bytes::Bytes;


/// Breaks down a `httparse::Request` struct into the core components
/// that we care about; method, path and headers.
pub fn request_to_parts(
    req: httparse::Request
) -> Result<(String, Bytes, HashMap<Bytes, Bytes>, usize), &'static str> {
    let method = req.method.unwrap().to_string();

    let path = req.path.unwrap().as_bytes();
    let path = Bytes::copy_from_slice(path);

    let mut headers: HashMap<Bytes, Bytes> = HashMap::new();
    for header in req.headers {
        headers.insert(
            Bytes::copy_from_slice(header.name.as_bytes()),
            Bytes::copy_from_slice(header.value),
        );
    }

    let val = headers.get(b"Content-Length".as_ref());

    let content_length = match val {
        Some(v) => {
            let res = match std::str::from_utf8(v.as_ref()) {
                Err(_) => return Err("bad request"),
                Ok(r) => r,
            };

            match res.parse() {
                Err(_) => return Err("bad request"),
                Ok(r) => r,
            }
        },
        _ => 0,
    };

    Ok((method, path, headers, content_length))
}

