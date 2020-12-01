
use bytes::BytesMut;
use std::iter::Enumerate;


static INVALID_CHUNK: &'static str = "Invalid Chunk";


/// The parse_chunked function takes a mutable reference to the main body,
/// it expects that the content is supposed to be chunked following the
/// correct syntax e.g. `2\r\nhi\r\n` anything other than this will be rejected
/// that being said the parser will return `Ok(None)` if it needs more body
/// to be able to get this chunk, if the chunk is valid and complete it
/// will return `Ok(Some(BytesMut))` where BytesMut is the stripped out
/// body part of the chunk.
/// If a chunk is invalid the function returns `Err(str)` all errors are the
/// same being that the chunk is invalid.
pub fn parse_chunked(mut body: &mut BytesMut) -> Result<Option<BytesMut>, &'static str> {
    if body.is_empty() {
        return Ok(None)
    }

    let expect_next: &[u8] = b"\r\n";
    let mut state: usize = 0;
    let mut split_at: usize = 0;

    let iter = body.iter().enumerate();
    for (i, part) in iter {
        let expect = expect_next[state];

        if (&expect == part) & { i == 0 } {
            return Err(INVALID_CHUNK)  // We got \r\n when we expected a hex
        } else if &expect == part {
            if state == 1 {
                split_at = i;
                break
            } else {
                state += 1
            }
        } else if (&expect != part) & (state != 0) {
            return Err(INVALID_CHUNK)
        }
    }

    if state != 3 {
        return Ok(None)
    }

    // Extract the length as hex + \r\n
    let mut specifier = body.split_to(split_at);

    // Remove the \r\n from the body
    let _ = specifier.split_off(split_at - 2);

    // Extract the content-length from from hex code
    let hex_code = String::from_utf8_lossy(specifier.as_ref());
    let amount = match usize::from_str_radix(&*hex_code, 16) {
        Err(_) => return Err(INVALID_CHUNK),
        Ok(n) => n,
    };

    // Body not long enough including the ending parts
    if body.len() < (amount + 2) {
        return Ok(None)
    }

    // Get the body + \r\n
    let mut resulting_body = body.split_to(amount + 2);

    // Get the \r\n and check if is what we expect
    let validator = resulting_body.split_off(2);
    if validator != expect_next {
        return Err(INVALID_CHUNK)
    }

    Ok(Some(resulting_body))
}
