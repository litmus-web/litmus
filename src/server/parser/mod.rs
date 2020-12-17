// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use crossbeam::channel;

use httparse::{
    self,
    Status::{Complete, Partial},
    parse_chunk_size
};

use http::header;
use http::Version;

use bytes::{BytesMut, BufMut};

use std::io::{Read, self};
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::mem;
use std::error::Error;

use fxhash::FxHashMap;


const MAX_BUFFER_SIZE: usize = 512 * 1024;  // 512 Kib
const MAX_HEADERS: usize = 100;
const ZERO_LENGTH: u8 = b"0"[0];


#[derive(Debug)]
pub struct Request {
    pub protocol: Version,
    pub path: String,
    pub headers: FxHashMap<String, Vec<u8>>,
    pub body_stream: channel::Receiver<(BytesMut, bool)>,

    pub keep_alive: bool,
}


/// The basis of this parser is to create a wrapper for httparse giving
/// us a much easier time when parsing pipelined requests, it contains it's
/// own buffer for parsing with a simple `H11Parser.read(&mut stream)` being
/// used to feed data to the parser, the parser has a self contained channel
/// where requests are buffered allowing multiple requests to be parsed
pub struct H11Parser {
    // State
    pub pause_reading: AtomicBool,
    expect_request: bool,
    chunked_encoding: bool,
    expected_length: usize,

    // Buffers
    internal_buffer: BytesMut,

    // Body streams
    body_in: channel::Sender<(BytesMut, bool)>,
    body_out: channel::Receiver<(BytesMut, bool)>,

    // Request queuing
    requests_in: channel::Sender<Request>,
    pub requests_out: channel::Receiver<Request>,
}

impl H11Parser {
    pub fn new(buffer_limit: usize) -> Self {
        let (requests_in, requests_out) = channel::bounded(5);
        let (body_in, body_out) = channel::bounded(5);

        Self {
            // State
            pause_reading: AtomicBool::new(false),
            expect_request: true,
            chunked_encoding: false,
            expected_length: 0,

            // Buffers
            internal_buffer: BytesMut::with_capacity(buffer_limit),

            // Body streams
            body_in,
            body_out,

            // Request queuing
            requests_in,
            requests_out,
        }
    }

    /// Reads the stream filling the internal parser buffer as much
    /// as it requires until the buffer is filled.
    pub fn read(&mut self, stream: &mut TcpStream) -> io::Result<bool> {
        let data = self.internal_buffer.bytes_mut();
        let slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(),data.len())
        };

        let len = stream.read(slice)?;

        unsafe { self.internal_buffer.advance_mut(len); }

        return Ok(self.internal_buffer.len() >= MAX_BUFFER_SIZE)
    }

    /// Begins parsing the body, this can produce many requests
    /// depending on the internal buffer, usually only one though if
    /// pipelining is not enabled.
    pub fn parse(&mut self) -> Result<(), ()> {
        if self.expect_request {
            match self.process_request() {
                Ok(_) => {},
                Err(_) => return Err(())
            }
        }

        return match self.feed_body() {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }


    fn process_request(&mut self) -> Result<(), Box<dyn Error>> {
        let mut headers: [httparse::Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let mut request = httparse::Request::new(&mut headers);

        let buffer = self.internal_buffer.clone();
        let status = request.parse(buffer.as_ref())?;

        // if its partial wait for the next round of parsing
        if status.is_partial() {
            return Ok(())
        }

        // Remove the header part of the buffer
        let n = status.unwrap();
        let _ = self.internal_buffer.split_to(n);

        // Handle the headers
        let req = self.process_headers(request)?;

        return match self.requests_in.try_send(req) {
            Ok(_) => Ok(()),
            Err(_) => panic!(
                "The request queue was full while trying to process a request.")
        }
    }

    fn process_headers(
        &mut self,
        req: httparse::Request
    ) -> Result<Request, Box<dyn Error>> {

        self.expected_length = 0;

        let (protocol, keep_alive) = if req.version == Some(0) {
            (Version::HTTP_10, false)
        } else {
            (Version::HTTP_11, true)
        };

        let path = req.path.unwrap_or("/").to_string();

        let mut headers = FxHashMap::default();
        for req_header in req.headers {
            if req_header.name == header::CONTENT_LENGTH {
                self.chunked_encoding = false;
                let temp = String::from_utf8_lossy(req_header.value);
                self.expected_length = temp.parse::<usize>()?
            } else if req_header.name == header::TRANSFER_ENCODING {
                let content = String::from_utf8_lossy(req_header.value);
                self.chunked_encoding = content.contains("chunked");
            }
            headers.insert(
                req_header.name.to_string(),
                req_header.value.to_vec()
            );
        }

        let request = Request {
            protocol,
            path,
            headers,
            body_stream: self.body_out.clone(),
            keep_alive,
        };

        Ok(request)
    }

    fn feed_body(&mut self) -> Result<(), httparse::InvalidChunkSize> {
        if !self.chunked_encoding {
            let check = self.internal_buffer.len();

            // We should fill the buffer more
            if (self.expected_length > check) & (check < MAX_BUFFER_SIZE) {
                return Ok(())
            }

            // Get the chunk of body, if it is able to split and have some
            // left over it means we have another request read.
            let (body, more_body) = if self.expected_length > check {
                self.expect_request = true;
                (self.internal_buffer.split_to(self.expected_length), true)
            } else {
                (self.internal_buffer.split_off(0), false)
            };

            self.send_body_or_panic(body, more_body);

            return Ok(())
        }

        return match parse_chunk_size(self.internal_buffer.as_ref())? {
            Complete((start, len)) => {
                let chunk = extract_body(
                    start,
                    len as usize,
                    &mut self.internal_buffer
                );

                self.send_body_or_panic(chunk, true);

                // No point checking for a ending chunk
                // and getting it's tail in the next read.
                if self.internal_buffer.len() < 5 {
                    return Ok(())
                }

                let len = match self.internal_buffer.get(0) {
                    None => return Ok(()),
                    Some(next) => next
                };

                // We got the 0\r\n\r\n meaning eof
                if len == &ZERO_LENGTH  {
                    let end = self.internal_buffer.split_to(5);
                    self.send_body_or_panic(end, false);
                    self.expect_request = true;
                }

                Ok(())
            },

            // We need more data
            Partial => Ok(())
        };
    }

    fn send_body_or_panic(&mut self, body: BytesMut, more_body: bool) {
        match self.body_in.try_send((body, more_body)) {
            Ok(_) => {},
            Err(_) => panic!(
                "The request queue was full while trying to process a request.")
        };
    }
}

fn extract_body(start: usize, len: usize, body: &mut BytesMut) -> BytesMut {
    let _ = body.split_to(start);
    let chunk = body.split_to(len as usize);
    let _ = body.split_to(2);

    chunk
}