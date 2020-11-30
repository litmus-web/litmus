// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use pyo3::PyResult;
use std::mem;
use std::sync::mpsc::{
    Receiver,
    SyncSender,
    sync_channel,
};
use bytes::BytesMut;
use http::header;


const MAX_HEADERS: usize = 48;


pub struct H11Parser {
    // State
    expecting_request: bool,
    chunked_encoding: bool,
    expected_length: usize,

    // Storage
    body_channel: Option<SyncSender<BytesMut>>,
}

impl H11Parser {
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
            }


        }

        Ok(Some(BytesMut))
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
            self.handle_request(request)?;
            Ok(Some(status.unwrap()))
        }
    }

    fn handle_request(&mut self, request: httparse::Request) -> Result<(), &'static str> {
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
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_body(&self) {}
}