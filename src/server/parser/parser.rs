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

use http::{
    header,
    uri,
};



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
        mut data: &BytesMut
    ) -> Result<Option<Receiver<BytesMut>>, &'static str> {

        if self.expecting_request {
            let n = match self.parse(&data)? {
                Some(n) => n,
                _ => return Ok(None)  // we need more data
            };
            self.expecting_request = false;

            let _ = data.split_to(n);

            if !self.chunked_encoding {
                let (tx, rx) = sync_channel(5);

                // Can we just get all the body and send it and be done?
                if data.len() >= self.expected_length {
                    let body = data.split_to(self.expected_length);
                    match tx.try_send(body) {
                        Ok(_) => {},
                        // This should never ever happen unless
                        // something is massively broken.
                        Err(_) => panic!("Channel was full upon sending.")
                    }
                }

                return Ok(Some(rx))
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
            self.submit_request(request);
            Ok(Some(status.unwrap()))
        }
    }

    fn process_headers(
        &mut self,
        request: &httparse::Request
    ) -> Result<Vec<(Py<PyBytes>, Py<PyBytes>)>, &'static str> {

        for header in request.headers {
            match header.name.parse::<header::HeaderName>().unwrap() {
                header::CONTENT_LENGTH => {
                    let len = String::from_utf8_lossy(header.value);
                    self.expected_length = match len.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => return Err("Invalid content length")
                    };
                    break;
                },
                header::TRANSFER_ENCODING => {
                    let value = String::from_utf8_lossy(header.value)
                        .to_lowercase();
                    if value.contains("chunked") {
                        self.chunked_encoding = true;
                    };
                    break;
                }
            };
        }

        let func = |py: Python| -> Vec<(Py<PyBytes>, Py<PyBytes>)> {
            let headers: Vec<(Py<PyBytes>, Py<PyBytes>)> = request.headers
                .iter()
                .map(|header| (
                    Py::from(PyBytes::new(py, header.name.as_bytes())),
                    Py::from(PyBytes::new(py, header.value.bytes())),
                    ))
                .collect();

            headers
        };

        Ok(Python::with_gil(func))
    }

    fn submit_request(&mut self, request: httparse::Request) {
        let headers = self.process_headers(&request)?;

        let path = request.path.unwrap_or("/");
        let (path, query) = match path.find("?") {
            Some(n) => path.split_at(n),
            _ => (path, "")
        };

        let req = Request {
            method: request.method.unwrap_or("GET").to_string(),
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