// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use pyo3::prelude::*;
use pyo3::types::PyBytes;

use std::mem;
use std::sync::mpsc::{
    Receiver,
    SyncSender,
    sync_channel,
};

use bytes::{BytesMut, Buf};

use http::header;

use super::chunked;


const MAX_HEADERS: usize = 48;


pub struct H11Parser {
    // State
    expecting_request: bool,
    chunked_encoding: bool,
    expected_length: usize,

    // Storage
    body_channel: Option<SyncSender<BytesMut>>,
    request_channel: SyncSender<Request>,
}

impl H11Parser {
    pub fn new(request_channel: SyncSender<Request>) -> Self {
        H11Parser {
            expecting_request: true,
            chunked_encoding: false,
            expected_length: 0,
            body_channel: None,
            request_channel,
        }
    }

    pub fn feed_data(
        &mut self,
        mut data: &mut BytesMut
    ) -> Result<Option<Receiver<BytesMut>>, &'static str> {

        if self.expecting_request {
            let n = match self.parse(&data)? {
                Some(n) => n,
                _ => return Ok(None)  // we need more data
            };
            self.expecting_request = false;

            let _ = data.split_to(n);

            let (tx, rx) = sync_channel(5);

            if !self.chunked_encoding {

                // Can we just get all the body and send it and be done?
                if data.len() >= self.expected_length {
                    let body = data.split_to(self.expected_length);
                    send(tx, body);
                    self.reset_state();
                } else {
                    // Send what's left
                    send(tx, data.clone());
                    data.clear();
                }

                match tx.try_send(body) {
                        Ok(_) => {},
                        // This should never ever happen unless
                        // something is massively broken.
                        Err(_) => panic!("Channel was full upon sending.")
                    }

                return Ok(Some(rx))
            } else {
                let maybe_body = match chunked::parse_chunked(data) {
                    Ok(b) => b,
                    Err(e) => return Err(e)
                };

                let body = match maybe_body {
                    Some(b) => b,
                    None => return Ok(None)
                };
            }
        }

        Ok(None)  // Get more data read to handle the next
    }

    fn reset_state(&mut self) {
        self.chunked_encoding = false;
        self.expected_length = 0;
        self.expecting_request = true;
    }

    fn parse(&mut self, mut data: &BytesMut) -> Result<Option<usize>, &'static str>  {
        let mut headers: [httparse::Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let mut request = httparse::Request::new(&mut headers);

        let status = match request.parse(data.as_ref()) {
            Ok(s) => s,
            Err(ref e) if httparse::Error::TooManyHeaders.eq(e) => {
                return Err("To many headers")
            },
            Err(_) => {
                return Err("Invalid request")
            }
        };

        return if status.is_partial() {
            Ok(None)
        } else {
            self.submit_request(request)?;
            Ok(Some(status.unwrap()))
        }
    }

    fn submit_request(
        &mut self, mut request: httparse::Request
    ) -> Result<(), &'static str> {
        let path = request.path.unwrap_or("/");
        let (path, query) = match path.find("?") {
            Some(n) => path.split_at(n),
            _ => (path, "")
        };

        let method = request.method.unwrap_or("GET").to_string();

        let headers = Python::with_gil(
            |py: Python| self.parse_headers(py, request)
        )?;

        let req = Request {
            method,
            path: path.to_string(),
            query: query.to_string(),
            headers
        };

        match self.request_channel.try_send(req) {
            Ok(_) => {},
            // This should never ever happen unless
            // something is massively broken.
            Err(_) => panic!("Channel was full upon sending.")
        }

        Ok(())
    }

    fn parse_headers(
        &mut self,
        py: Python,
        request: httparse::Request
    ) -> Result<Vec<(Py<PyBytes>, Py<PyBytes>)>, &'static str> {
        let mut new = Vec::new();
        for req_header in request.headers {
            match req_header.name.parse::<header::HeaderName>().unwrap() {
                header::CONTENT_LENGTH => {
                    let len = String::from_utf8_lossy(req_header.value);
                    self.expected_length = match len.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => return Err("Invalid content length")
                    };
                },
                header::TRANSFER_ENCODING => {
                    let value = String::from_utf8_lossy(req_header.value)
                        .to_lowercase();
                    if value.contains("chunked") {
                        self.chunked_encoding = true;
                    };
                }
                _ => {},
            };

            new.push((
                Py::from(PyBytes::new(py, req_header.name.as_bytes())),
                Py::from(PyBytes::new(py, req_header.value.bytes())),
            ));
        }

        Ok(new)
    }

    fn handle_body(&self) {

    }
}


pub struct Request {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: Vec<(Py<PyBytes>, Py<PyBytes>)>,
}


fn send(tx: SyncSender<BytesMut>, data: BytesMut) {
    match tx.try_send(data) {
        Ok(_) => {},
        // This should never ever happen unless
        // something is massively broken.
        Err(_) => panic!("Channel was full upon sending.")
    }
}
