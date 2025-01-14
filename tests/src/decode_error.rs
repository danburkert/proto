#![cfg(test)]

use alloc::{boxed::Box, string::ToString, vec::Vec};
use prost::Message;
use protobuf::test_messages::proto3::TestAllTypesProto3;

#[test]
fn test_decode_error_invalid_wire_type() {
    let msg = [0x36].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid wire type value: 6"
    );
}

#[test]
fn test_decode_error_invalid_varint() {
    let msg = [0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.optional_int32: invalid varint"
    );
}

#[test]
fn test_decode_error_multiple_levels() {
    use protobuf::test_messages::proto3::ForeignMessage;
    let msg = TestAllTypesProto3 {
        recursive_message: Some(Box::new(TestAllTypesProto3 {
            optional_foreign_message: Some(ForeignMessage { c: -1 }),
            ..Default::default()
        })),
        ..Default::default()
    };
    let mut buf = msg.encode_to_vec();

    // Last byte is part of varint value `-1`. Set it to an invalid value.
    assert_eq!(buf.last().unwrap(), &0x01);
    *buf.last_mut().unwrap() = 0xFF;

    assert_eq!(
            TestAllTypesProto3::decode(buf.as_slice()).unwrap_err().to_string(),
            "failed to decode Protobuf message: ForeignMessage.c: TestAllTypesProto3.optional_foreign_message: TestAllTypesProto3.recursive_message: invalid varint"
        );
}

#[cfg(not(target_pointer_width = "64"))]
#[test]
fn test_decode_error_length_delimiter_too_large() {
    assert!((usize::MAX as u64) < u64::MAX);

    let msg = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01].as_slice();
    assert_eq!(
        prost::decode_length_delimiter(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: length delimiter exceeds maximum usize value"
    );
}

#[test]
fn test_decode_error_recursion_limit_reached() {
    let recursve_message = {
        let mut msg = TestAllTypesProto3::default();
        for _ in 0..101 {
            msg = TestAllTypesProto3 {
                recursive_message: Some(Box::new(msg)),
                ..Default::default()
            };
        }
        msg
    };

    let buf = recursve_message.encode_to_vec();
    assert_eq!(
        TestAllTypesProto3::decode(buf.as_slice()).unwrap_err().to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: recursion limit reached"
    );
}

#[test]
fn test_decode_error_invalid_key_value() {
    let msg = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid key value: 1125899906842623"
    );
}

#[test]
fn test_decode_error_invalid_tag() {
    let msg = [0x00].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid tag value: 0"
    );
}

#[test]
fn test_decode_error_unexpected_wire_type() {
    let mut buf = [0x00].as_slice();
    let mut msg = TestAllTypesProto3::default();
    let ctx = prost::encoding::DecodeContext::default();
    assert_eq!(
        msg.merge_field(1, prost::encoding::WireType::LengthDelimited, &mut buf, ctx).unwrap_err().to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.optional_int32: invalid wire type: LengthDelimited (expected Varint)"
    );
}

#[test]
fn test_decode_error_buffer_underflow() {
    let msg = [0x12].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode_length_delimited(msg)
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: buffer underflow"
    );
}

#[test]
fn test_decode_error_invalid_string() {
    let msg = TestAllTypesProto3 {
        optional_string: "Hello".to_string(),
        ..Default::default()
    };
    let mut buf = msg.encode_to_vec();

    // Last byte is part of string value `o`. Set it to an invalid value.
    assert_eq!(buf.last().unwrap(), &b'o');
    *buf.last_mut().unwrap() = 0xA0;

    assert_eq!(
            TestAllTypesProto3::decode(buf.as_slice()).unwrap_err().to_string(),
            "failed to decode Protobuf message: TestAllTypesProto3.optional_string: invalid string value: data is not UTF-8 encoded"
        );
}

#[test]
fn test_decode_error_any() {
    use prost_types::{Any, Timestamp};

    let msg = Any {
        type_url: "non-existing-url".to_string(),
        value: Vec::new(),
    };

    assert_eq!(
        msg.to_msg::<Timestamp>().unwrap_err().to_string(),
            "failed to decode Protobuf message: unexpected type URL.type_url: expected type URL: \"type.googleapis.com/google.protobuf.Timestamp\" (got: \"non-existing-url\")"
        );
}
