// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use std::error::Error;
use std::mem;
use std::str::FromStr;

use bytes::{BytesMut, Bytes, Buf, BufMut};

use crossbeam::channel::{unbounded, Receiver, Sender};

use httparse::{Status, parse_chunk_size, Header};
use http::method::Method;



const MAX_HEADERS: usize = 100;


pub enum ParserStatus {
    NoEnoughData,
}


pub struct Request {
    version: u8,
    status: String,
    path: String,
}


pub fn extract_request(buffer: &mut BytesMut) -> Result<ParserStatus, Box<dyn Error>> {
    let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
        mem::uninitialized()
    };

    let mut request = httparse::Request::new(&mut headers);

    let buff_copy = buffer.clone();
    let status = request.parse(&buff_copy)?;

    if status.is_partial() {
        return Ok(ParserStatus::NoEnoughData)
    }

    let split_at = status.unwrap();
    let _ = buffer.split_to(split_at);

    let status = request.method
        .expect("Method was None after complete parse")
        .to_string();
    let path = request.path
        .expect("Path was None after complete parse")
        .to_string();
    let version = request.version.
        expect("Version was None after complete parse");


    for header in headers.iter() {
        println!("{:?}", &header.name);
    }

    //println!("{:?}", &request.headers);


    Ok(ParserStatus::NoEnoughData)
}