//! Reader for Dota 2's local "VBKV" binary KeyValues format, used by
//! `userdata/<id>/570/remote/cfg/{stats,last_match}.dat`.
//!
//! Layout (little-endian):
//!   magic:  4 bytes  b"VBKV"
//!   header: 4 bytes  (checksum/hash - unused)
//!   body:   a single dict node
//!
//! Type bytes:
//!   0x00 = nested dict (recurse until a 0x0B terminator)
//!   0x01 = string (null-terminated)
//!   0x02 = i32 (4 bytes LE, signed)
//!   0x03 = f32 (4 bytes LE)
//!   0x07 = u64 (8 bytes LE)
//!   0x0B = end-of-dict marker

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Dict(BTreeMap<String, Value>),
    Str(String),
    I32(i32),
    F32(f32),
    U64(u64),
}

impl Value {
    pub fn as_dict(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Dict(d) => Some(d),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_dict()?.get(key)
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(v) => Some(*v),
            _ => None,
        }
    }

    /// Converts the parsed tree to `serde_json::Value`, for `dotafetch dump`
    /// - a human-readable view of any of these `.dat` files for debugging.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Dict(d) => {
                serde_json::Value::Object(d.iter().map(|(k, v)| (k.clone(), v.to_json())).collect())
            }
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::I32(n) => serde_json::Value::from(*n),
            Value::F32(f) => serde_json::Value::from(*f),
            Value::U64(n) => serde_json::Value::from(*n),
        }
    }
}

#[derive(Debug)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vbkv parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

fn err(msg: impl Into<String>) -> ParseError {
    ParseError(msg.into())
}

fn read_cstr(buf: &[u8], pos: &mut usize) -> Result<String, ParseError> {
    let start = *pos;
    let end = buf[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|i| start + i)
        .ok_or_else(|| err(format!("unterminated string at offset {start}")))?;
    *pos = end + 1;
    Ok(String::from_utf8_lossy(&buf[start..end]).into_owned())
}

fn take<'a>(buf: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8], ParseError> {
    if *pos + n > buf.len() {
        return Err(err(format!(
            "unexpected end of buffer at offset {} (need {n} bytes)",
            *pos
        )));
    }
    let slice = &buf[*pos..*pos + n];
    *pos += n;
    Ok(slice)
}

fn parse_dict(buf: &[u8], pos: &mut usize) -> Result<BTreeMap<String, Value>, ParseError> {
    let mut result = BTreeMap::new();
    loop {
        if *pos >= buf.len() {
            return Err(err("unexpected end of buffer while reading dict"));
        }
        let t = buf[*pos];
        *pos += 1;
        if t == 0x0B {
            return Ok(result);
        }
        let key = read_cstr(buf, pos)?;
        let value = match t {
            0x00 => Value::Dict(parse_dict(buf, pos)?),
            0x01 => Value::Str(read_cstr(buf, pos)?),
            0x02 => {
                let bytes: [u8; 4] = take(buf, pos, 4)?.try_into().unwrap();
                Value::I32(i32::from_le_bytes(bytes))
            }
            0x03 => {
                let bytes: [u8; 4] = take(buf, pos, 4)?.try_into().unwrap();
                Value::F32(f32::from_le_bytes(bytes))
            }
            0x07 => {
                let bytes: [u8; 8] = take(buf, pos, 8)?.try_into().unwrap();
                Value::U64(u64::from_le_bytes(bytes))
            }
            other => {
                return Err(err(format!(
                    "unknown type byte 0x{other:02x} at offset {} (key {key:?})",
                    *pos - 1
                )));
            }
        };
        result.insert(key, value);
    }
}

pub fn parse(buf: &[u8]) -> Result<Value, ParseError> {
    if buf.len() < 8 || &buf[0..4] != b"VBKV" {
        return Err(err("missing VBKV magic header"));
    }
    let mut pos = 8; // magic + 4-byte header
    Ok(Value::Dict(parse_dict(buf, &mut pos)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real 57-byte last_match.dat captured from a live account.
    const LAST_MATCH_FIXTURE: &[u8] = &[
        0x56, 0x42, 0x4b, 0x56, 0x73, 0xc7, 0x66, 0xbf, 0x00, 0x4c, 0x61, 0x73, 0x74, 0x4d, 0x61,
        0x74, 0x63, 0x68, 0x00, 0x07, 0x6c, 0x61, 0x73, 0x74, 0x5f, 0x6d, 0x61, 0x74, 0x63, 0x68,
        0x5f, 0x69, 0x64, 0x00, 0xa8, 0x22, 0x5d, 0x18, 0x02, 0x00, 0x00, 0x00, 0x00, 0x6d, 0x61,
        0x74, 0x63, 0x68, 0x5f, 0x64, 0x61, 0x74, 0x61, 0x00, 0x0b, 0x0b, 0x0b,
    ];

    #[test]
    fn rejects_bad_magic() {
        let err = parse(b"NOPE0000").unwrap_err();
        assert!(err.to_string().contains("magic"));
    }

    #[test]
    fn rejects_truncated_buffer() {
        let truncated = &LAST_MATCH_FIXTURE[..LAST_MATCH_FIXTURE.len() - 5];
        assert!(parse(truncated).is_err());
    }
}
