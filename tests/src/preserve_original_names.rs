pub mod original_names {
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/preserve_original_names.rs"));
}

#[test]
fn test() {
    use alloc::vec::Vec;
    use prost::{bytes::Bytes, Message};

    let _ = original_names::NoConflictsInEnumNames {
        field2: original_names::Enum1_2::A1_2.into(),
        field1: original_names::Enum12::A12.into(),
    };

    let value = original_names::IPPrefix {
        len: 24,
        prefix: "192.168.2.1".into(),
        ip_version: original_names::IPVersion::IP_V4.into(),
    };

    let mut buf: Vec<u8> = Vec::new();
    value.encode(&mut buf).unwrap();
    let unpacked = Message::decode(Bytes::from(buf)).unwrap();

    assert_eq!(value, unpacked);
}
