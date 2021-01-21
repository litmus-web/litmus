// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use std::error::Error;
use std::mem;

use bytes::{BytesMut, Bytes, Buf, BufMut};

use crossbeam::channel::{unbounded, Receiver, Sender};

use httparse::{Status, parse_chunk_size, Header};


const MAX_HEADERS: usize = 100;


pub enum ParserStatus {
    NoEnoughData,
}


pub struct Request {

}


pub fn extract_request(buffer: &mut BytesMut) -> Result<[Header<'_>; MAX_HEADERS], Box<dyn Error>> {
    let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
        mem::uninitialized()
    };

    let mut request = httparse::Request::new(&mut headers);

    let status = request.parse(buffer)?;


    Ok(headers)
}