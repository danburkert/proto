//! Tests for our own Debug implementation.
//!
//! The tests check against expected output. This may be a bit fragile, but it is likely OK for
//! actual use.

use prost::alloc::{format, string::String};

// Borrow some types from other places.
#[cfg(feature = "std")]
use crate::message_encoding::Basic;
use crate::message_encoding::BasicEnumeration;

/// Some real-life message
#[test]
#[cfg(feature = "std")]
fn basic() {
    let mut basic = Basic::default();
    assert_eq!(
        format!("{:?}", basic),
        "Basic { \
         int32: 0, \
         bools: [], \
         string: \"\", \
         optional_string: None, \
         enumeration: ZERO, \
         enumeration_map: {}, \
         string_map: {}, \
         enumeration_btree_map: {}, \
         string_btree_map: {}, \
         oneof: None, \
         bytes_map: {} \
         }"
    );
    basic
        .enumeration_map
        .insert(0, BasicEnumeration::TWO as i32);
    basic.enumeration = 42;
    basic
        .bytes_map
        .insert("hello".to_string(), "world".as_bytes().into());
    assert_eq!(
        format!("{:?}", basic),
        "Basic { \
         int32: 0, \
         bools: [], \
         string: \"\", \
         optional_string: None, \
         enumeration: 42, \
         enumeration_map: {0: TWO}, \
         string_map: {}, \
         enumeration_btree_map: {}, \
         string_btree_map: {}, \
         oneof: None, \
         bytes_map: {\"hello\": [119, 111, 114, 108, 100]} \
         }"
    );
}

/// A special case with a tuple struct
#[test]
fn tuple_struct() {
    #[derive(Clone, PartialEq, prost::Message)]
    struct NewType(#[prost(enumeration = "BasicEnumeration", tag = "5")] i32);
    assert_eq!(
        format!("{:?}", NewType(BasicEnumeration::TWO as i32)),
        "NewType(TWO)"
    );
    assert_eq!(format!("{:?}", NewType(42)), "NewType(42)");
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, prost::Oneof)]
pub enum OneofWithEnum {
    #[prost(int32, tag = "8")]
    Int(i32),
    #[prost(string, tag = "9")]
    String(String),
    #[prost(enumeration = "BasicEnumeration", tag = "10")]
    Enumeration(i32),
}

#[derive(Clone, PartialEq, prost::Message)]
struct MessageWithOneof {
    #[prost(oneof = "OneofWithEnum", tags = "8, 9, 10")]
    of: Option<OneofWithEnum>,
}

/// Enumerations inside oneofs
#[test]
fn oneof_with_enum() {
    let msg = MessageWithOneof {
        of: Some(OneofWithEnum::Enumeration(BasicEnumeration::TWO as i32)),
    };
    assert_eq!(
        format!("{:?}", msg),
        "MessageWithOneof { of: Some(Enumeration(TWO)) }"
    );
}
