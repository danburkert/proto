#![doc(html_root_url = "https://docs.rs/prost/0.4.0")]

extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod error;
mod message;
mod types;

#[doc(hidden)]
pub mod encoding;

use std::cmp;
use std::io::{Cursor, Read};
use std::marker::PhantomData;

pub use error::{DecodeError, EncodeError};
pub use message::Message;

use bytes::{Buf, BufMut, IntoBuf};

use encoding::{decode_varint, encode_varint, encoded_len_varint};

/// Encodes a length delimiter to the buffer.
///
/// See [Message.encode_length_delimited] for more info.
///
/// An error will be returned if the buffer does not have sufficient capacity to encode the
/// delimiter.
pub fn encode_length_delimiter<B>(length: usize, buf: &mut B) -> Result<(), EncodeError>
where
    B: BufMut,
{
    let length = length as u64;
    let required = encoded_len_varint(length);
    let remaining = buf.remaining_mut();
    if required > remaining {
        return Err(EncodeError::new(required, remaining));
    }
    encode_varint(length, buf);
    Ok(())
}

/// Returns the encoded length of a length delimiter.
///
/// Applications may use this method to ensure sufficient buffer capacity before calling
/// `encode_length_delimiter`. The returned size will be between 1 and 10, inclusive.
pub fn length_delimiter_len(length: usize) -> usize {
    encoded_len_varint(length as u64)
}

/// Decodes a length delimiter from the buffer.
///
/// This method allows the length delimiter to be decoded independently of the message, when the
/// message is encoded with [Message.encode_length_delimited].
///
/// An error may be returned in two cases:
///
///  * If the supplied buffer contains fewer than 10 bytes, then an error indicates that more
///    input is required to decode the full delimiter.
///  * If the supplied buffer contains more than 10 bytes, then the buffer contains an invalid
///    delimiter, and typically the buffer should be considered corrupt.
pub fn decode_length_delimiter<B>(buf: B) -> Result<usize, DecodeError>
where
    B: IntoBuf,
{
    let mut buf = buf.into_buf();
    let length = decode_varint(&mut buf)?;
    if length > usize::max_value() as u64 {
        return Err(DecodeError::new(
            "length delimiter exceeds maximum usize value",
        ));
    }
    Ok(length as usize)
}

// TODO: Something better, probably an enum between io-error and DecodeError
type E = Box<dyn std::error::Error + Send + Sync>;

// TODO: Tests for the stuff
// TODO: Docs, examples
// TODO: Do we want to be able to write an iterator to a file or buffer?
// TODO: How to expose this? As methods on Message, or free-standing functions?

pub struct Decoder<R> {
    read: R,
    buffer: Vec<u8>,
}

pub const MAX_DELIMITER_LEN: usize = 10;

impl<R: Read> Decoder<R> {
    pub fn new(read: R) -> Self {
        Self {
            read,
            buffer: Vec::new(),
        }
    }

    fn refill(&mut self, at_least: usize) -> Result<(), E> {
        let needed = at_least - cmp::min(at_least, self.buffer.len());
        self.buffer.reserve(needed);
        self.read.by_ref().take(needed as u64).read_to_end(&mut self.buffer)?;
        Ok(())
    }

    fn shift(&mut self, by: usize) {
        let new_size = self.buffer.len() - by;
        for i in 0..new_size {
            self.buffer[i] = self.buffer[i + by];
        }
        self.buffer.truncate(new_size);
    }

    pub fn decode<M: Default + Message>(&mut self) -> Result<Option<M>, E> {
        self.refill(MAX_DELIMITER_LEN)?;
        if self.buffer.is_empty() {
            return Ok(None);
        }
        let msg_len;
        let skip;
        {
            let mut buffer = Cursor::new(&self.buffer[..]);
            msg_len = decode_length_delimiter(&mut buffer)?;
            skip = buffer.position() as usize;
        }
        self.shift(skip);
        self.refill(msg_len + MAX_DELIMITER_LEN)?;
        if self.buffer.len() < msg_len {
            return Err(DecodeError::new("input buffer too short").into());
        }
        let msg = M::decode(&self.buffer[..msg_len])?;
        self.shift(msg_len);
        Ok(Some(msg))
    }

    pub fn decode_all<M: Default + Message>(self) -> DecodeAll<R, M> {
        DecodeAll {
            inner: self,
            _msg: PhantomData,
        }
    }
}

pub struct DecodeAll<R, M> {
    inner: Decoder<R>,
    _msg: PhantomData<fn() -> M>,
}

impl<R: Read, M: Default + Message> Iterator for DecodeAll<R, M> {
    type Item = Result<M, E>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.decode() {
            Ok(None) => None,
            Ok(Some(m)) => Some(Ok(m)),
            Err(e) => Some(Err(e)),
        }
    }
}

pub struct DecodeBuf<B, M> {
    buf: B,
    _msg: PhantomData<fn() -> M>,
}

impl<B: Buf, M: Default + Message> Iterator for DecodeBuf<B, M> {
    type Item = Result<M, DecodeError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.remaining() == 0 {
            None
        } else {
            Some(M::decode_length_delimited(&mut self.buf))
        }
    }
}

pub fn decode_buf<B: IntoBuf, M: Default + Message>(buf: B) -> DecodeBuf<B::Buf, M> {
    DecodeBuf {
        buf: buf.into_buf(),
        _msg: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_encoded() -> Vec<u8> {
        let mut buf = Vec::new();
        "hello".to_owned().encode_length_delimited(&mut buf).unwrap();
        "world".to_owned().encode_length_delimited(&mut buf).unwrap();
        buf
    }

    #[test]
    fn encode_decode_buf() {
        let decoded = decode_buf(get_encoded())
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        assert_eq!(vec!["hello".to_owned(), "world".to_owned()], decoded);
    }

    #[test]
    fn encode_decode_read() {
        let decoded = Decoder::new(&get_encoded() as &[u8])
            .decode_all()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        assert_eq!(vec!["hello".to_owned(), "world".to_owned()], decoded);
    }
}
