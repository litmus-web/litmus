
use bytes::BytesMut;
use std::iter::Enumerate;


static INVALID_CHUNK: &'static str = "Invalid Chunk";


fn parse_chunked(mut body: BytesMut) -> Result<Option<(usize, BytesMut)>, &'static str> {
    if body.is_empty() {
        return Ok(None)
    }

    let expect_next: &[u8] = b"\r\n\r\n";
    let mut state: usize = 0;
    let mut split_at: usize = 0;

    for (i, part) in body.iter().enumerate() {
        let expect = expect_next[state];

        if (&expect == part) & { i == 0 } {
            return Err(INVALID_CHUNK)
        } else if &expect == part {
            if state == 3 {
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

    let mut specifier = body.split_to(split_at);
    let _ = specifier.split_off(4);
    let hex_code = String::from_utf8_lossy(specifier.as_ref());
    let amount = match usize::from_str_radix(&*hex_code, 16) {
        Err(_) => return Err(INVALID_CHUNK),
        Ok(n) => n,
    };

    let value =  if amount >= body.len() {
        Some((amount + specifier.len() + 4, body.split_to(amount)))
    } else {
        None
    };

    Ok(value)
}
