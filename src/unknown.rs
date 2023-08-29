use alloc::vec::Vec;

use core::fmt::Debug;
use core::usize;

use bytes::{Buf, BufMut};

use crate::encoding::{decode_varint, encode_key, encode_varint, key_len, WireType};
use crate::DecodeError;

pub struct UnknownField {
    tag: u32,
    wire_type: WireType,
    bytes: Vec<u8>,
}

impl Debug for UnknownField {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UnknownField")
            .field("tag", &self.tag)
            .field("wire_type", &self.wire_type)
            .field("bytes", &self.bytes)
            .finish()
    }
}

impl Clone for UnknownField {
    fn clone(&self) -> Self {
        Self {
            tag: self.tag.clone(),
            wire_type: self.wire_type.clone(),
            bytes: self.bytes.clone(),
        }
    }
}

impl PartialEq for UnknownField {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.wire_type == other.wire_type && self.bytes == other.bytes
    }
}

impl UnknownField {
    fn encoded_len(&self) -> usize {
        key_len(self.tag) + self.bytes.len()
    }
}

pub struct UnknownFields {
    fields: Vec<UnknownField>,
}

impl Debug for UnknownFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UnknownFields")
            .field("fields", &self.fields)
            .finish()
    }
}

impl Clone for UnknownFields {
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
        }
    }
}

impl PartialEq for UnknownFields {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

impl Eq for UnknownFields {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Default for UnknownFields {
    fn default() -> Self {
        Self { fields: Vec::new() }
    }
}

impl UnknownFields {
    pub fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        for field in self.fields.iter() {
            encode_key(field.tag, field.wire_type, buf);
            if WireType::LengthDelimited == field.wire_type {
                encode_varint(field.bytes.len() as u64, buf);
            }
            buf.put(&field.bytes[..]);
        }
    }
    pub fn merge_next_field<B: Buf>(
        &mut self,
        wire_type: WireType,
        tag: u32,
        buf: &mut B,
    ) -> Result<(), DecodeError> {
        let bytes = match wire_type {
            WireType::Varint => {
                let v = decode_varint(buf)?;
                let mut bytes = Vec::new();
                encode_varint(v, &mut bytes);
                bytes
            }
            WireType::ThirtyTwoBit => {
                let mut bytes = Vec::with_capacity(4);
                let mut take = buf.take(4);
                bytes.put(&mut take);
                bytes
            }
            WireType::SixtyFourBit => {
                let mut bytes = Vec::with_capacity(8);
                let mut take = buf.take(8);
                bytes.put(&mut take);
                bytes
            }
            WireType::LengthDelimited => {
                let len = decode_varint(buf)? as usize;
                let mut bytes = Vec::with_capacity(len);
                let mut take = buf.take(len);
                bytes.put(&mut take);
                bytes
            }
            // TODO(jason)
            WireType::StartGroup => unimplemented!(),
            WireType::EndGroup => unimplemented!(),
        };

        self.fields.push(UnknownField {
            tag,
            wire_type,
            bytes,
        });

        Ok(())
    }
    pub fn encoded_len(&self) -> usize {
        self.fields.iter().map(|f| f.encoded_len()).sum()
    }
    pub fn clear(&mut self) {
        self.fields.clear()
    }
}
