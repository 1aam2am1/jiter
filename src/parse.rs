use std::ops::Range;

use crate::JsonError;

pub fn parse_string(data: &[u8], range: Range<usize>) -> Result<String, JsonError> {
    let mut chars = Vec::with_capacity(range.end - range.start);
    let mut index = range.start;
    if data.len() < range.end {
        return Err(JsonError::InternalError);
    }
    while index < range.end {
        // we can safely do ths as we know the logic in chunk...parse_string would have raised
        // an error if we were at the end of the string
        let next = unsafe { data.get_unchecked(index) };
        match next {
            b'\\' => {
                index += 1;
                // again we can safely get the next byte
                let next = unsafe { data.get_unchecked(index) };
                match next {
                    b'"' | b'\\' | b'/' => chars.push(*next),
                    b'b' => chars.push(b'\x08'),
                    b'f' => chars.push(b'\x0C'),
                    b'n' => chars.push(b'\n'),
                    b'r' => chars.push(b'\r'),
                    b't' => chars.push(b'\t'),
                    b'u' => {
                        index += 1;
                        if index + 3 >= range.end {
                            return Err(JsonError::InvalidString(index - range.start));
                        }
                        let c16 = decode_hex_escape(data, index, &range)?;
                        match char::from_u32(c16 as u32) {
                            Some(c) => {
                                for b in c.to_string().bytes() {
                                    chars.push(b);
                                }
                            }
                            None => return Err(JsonError::InvalidString(index - range.start)),
                        }
                        index += 3;
                    }
                    _ => return Err(JsonError::InvalidString(index - range.start)),
                }
            }
            _ => chars.push(*next),
        }
        index += 1;
    }
    Ok(String::from_utf8(chars).map_err(|_| JsonError::InternalError)?)
}

/// borrowed from serde-json unless we can do something faster?
fn decode_hex_escape(data: &[u8], index: usize, range: &Range<usize>) -> Result<u16, JsonError> {
    let mut n = 0;
    for i in 0..4 {
        let c = unsafe { data.get_unchecked(index + i) };
        let hex = match c {
            b'0'..=b'9' => (c & 0x0f) as u16,
            b'a'..=b'f' => (c - b'a' + 10) as u16,
            b'A'..=b'F' => (c - b'A' + 10) as u16,
            _ => return Err(JsonError::InvalidStringEscapeSequence(index + i - range.start)),
        };
        n = (n << 4) + hex;
    }
    Ok(n)
}

pub fn parse_int(data: &[u8], positive: bool, range: Range<usize>) -> Result<i64, JsonError> {
    let mut result: u64 = 0;
    if data.len() < range.end {
        return Err(JsonError::InternalError);
    }
    for index in range {
        let digit = unsafe { data.get_unchecked(index) };
        match digit {
            b'0'..=b'9' => {
                result *= 10;
                result += (digit & 0x0f) as u64;
                if result >= i64::MAX as u64 {
                    return Err(JsonError::IntTooLarge);
                }
            }
            _ => return Err(JsonError::InvalidNumber),
        }
    }
    if positive {
        Ok(result as i64)
    } else {
        Ok(-(result as i64))
    }
}

pub fn parse_float(
    data: &[u8],
    positive: bool,
    int_range: Range<usize>,
    decimal_range: Range<usize>,
) -> Result<f64, JsonError> {
    let mut result = parse_int(data, true, int_range)? as f64;
    if data.len() < decimal_range.end {
        return Err(JsonError::InternalError);
    }
    for (pos, index) in decimal_range.enumerate() {
        let digit = unsafe { data.get_unchecked(index) };
        match digit {
            b'0'..=b'9' => {
                result += (digit & 0x0f) as f64 / 10f64.powi(pos as i32 + 1);
            }
            _ => return Err(JsonError::InvalidNumber),
        }
    }
    if positive {
        Ok(result)
    } else {
        Ok(-result)
    }
}