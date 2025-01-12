#![allow(
    clippy::cognitive_complexity,
    clippy::module_inception,
    clippy::unreadable_literal
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate cfg_if;

extern crate alloc;

cfg_if! {
    if #[cfg(feature = "edition-2015")] {
        extern crate anyhow;
        extern crate core;
        extern crate prost;
        extern crate prost_types;
        extern crate protobuf;
        #[cfg(test)]
        extern crate prost_build;
        #[cfg(test)]
        extern crate tempfile;
    }
}

pub mod decode_error;
pub mod extern_paths;
pub mod no_root_packages;
pub mod packages;
pub mod unittest;

#[cfg(test)]
mod bootstrap;
#[cfg(test)]
mod debug;
#[cfg(test)]
mod deprecated_field;
#[cfg(test)]
mod derive_copy;
#[cfg(test)]
mod enum_keyword_variant;
#[cfg(test)]
mod generic_derive;
#[cfg(test)]
mod message_encoding;
#[cfg(test)]
mod no_shadowed_types;
#[cfg(test)]
mod no_unused_results;
#[cfg(test)]
mod submessage_without_package;
#[cfg(test)]
mod type_names;

#[cfg(test)]
mod boxed_field;

#[cfg(test)]
mod custom_debug;

// Must be `pub` as doc tests are only executed on public types.
pub mod disable_comments;

#[cfg(test)]
// Must be `pub` as `missing_docs` lint is only executed on public types.
pub mod custom_attributes;

#[cfg(test)]
mod default_enum_value;

#[cfg(test)]
mod nesting;

mod test_enum_named_option_value {
    include!(concat!(env!("OUT_DIR"), "/myenum.optionn.rs"));
}

mod test_enum_named_result_value {
    include!(concat!(env!("OUT_DIR"), "/myenum.result.rs"));
}

mod test_result_named_option_value {
    include!(concat!(env!("OUT_DIR"), "/mystruct.optionn.rs"));
}

mod test_result_named_result_value {
    include!(concat!(env!("OUT_DIR"), "/mystruct.result.rs"));
}

pub mod foo {
    pub mod bar_baz {
        include!(concat!(env!("OUT_DIR"), "/foo.bar_baz.rs"));
    }
}

pub mod recursive_oneof {
    include!(concat!(env!("OUT_DIR"), "/recursive_oneof.rs"));
}

/// Also for testing custom attributes, but on oneofs.
///
/// Unfortunately, an OneOf field generates a companion module in the .rs file. There's no
/// reasonable way to place a doc comment on that, so we do the test with `derive(Ord)` and have it
/// in a separate file.
pub mod oneof_attributes {
    include!(concat!(env!("OUT_DIR"), "/foo.custom.one_of_attrs.rs"));
}

pub mod groups {
    include!(concat!(env!("OUT_DIR"), "/groups.rs"));
}

pub mod proto3 {
    pub mod presence {
        include!(concat!(env!("OUT_DIR"), "/proto3.presence.rs"));
    }
}

pub mod default_string_escape {
    include!(concat!(env!("OUT_DIR"), "/default_string_escape.rs"));
}

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use anyhow::anyhow;
use prost::bytes::Buf;

use prost::Message;

pub enum RoundtripResult {
    /// The roundtrip succeeded.
    Ok(Vec<u8>),
    /// The data could not be decoded. This could indicate a bug in prost,
    /// or it could indicate that the input was bogus.
    DecodeError(prost::DecodeError),
    /// Re-encoding or validating the data failed.  This indicates a bug in `prost`.
    Error(anyhow::Error),
}

impl RoundtripResult {
    /// Unwrap the roundtrip result.
    pub fn unwrap(self) -> Vec<u8> {
        match self {
            RoundtripResult::Ok(buf) => buf,
            RoundtripResult::DecodeError(error) => {
                panic!("failed to decode the roundtrip data: {}", error)
            }
            RoundtripResult::Error(error) => panic!("failed roundtrip: {}", error),
        }
    }

    /// Unwrap the roundtrip result. Panics if the result was a validation or re-encoding error.
    pub fn unwrap_error(self) -> Result<Vec<u8>, prost::DecodeError> {
        match self {
            RoundtripResult::Ok(buf) => Ok(buf),
            RoundtripResult::DecodeError(error) => Err(error),
            RoundtripResult::Error(error) => panic!("failed roundtrip: {}", error),
        }
    }
}

/// Tests round-tripping a message type. The message should be compiled with `BTreeMap` fields,
/// otherwise the comparison may fail due to inconsistent `HashMap` entry encoding ordering.
pub fn roundtrip<M>(data: &[u8]) -> RoundtripResult
where
    M: Message + Default,
{
    // Try to decode a message from the data. If decoding fails, continue.
    let all_types = match M::decode(data) {
        Ok(all_types) => all_types,
        Err(error) => return RoundtripResult::DecodeError(error),
    };

    let encoded_len = all_types.encoded_len();

    // TODO: Reenable this once sign-extension in negative int32s is figured out.
    // assert!(encoded_len <= data.len(), "encoded_len: {}, len: {}, all_types: {:?}",
    //         encoded_len, data.len(), all_types);

    let mut buf1 = Vec::new();
    if let Err(error) = all_types.encode(&mut buf1) {
        return RoundtripResult::Error(anyhow!(error));
    }
    let buf1 = buf1;
    if encoded_len != buf1.len() {
        return RoundtripResult::Error(anyhow!(
            "expected encoded len ({}) did not match actual encoded len ({})",
            encoded_len,
            buf1.len()
        ));
    }

    let roundtrip = match M::decode(buf1.as_slice()) {
        Ok(roundtrip) => roundtrip,
        Err(error) => return RoundtripResult::Error(anyhow!(error)),
    };

    let mut buf2 = Vec::new();
    if let Err(error) = roundtrip.encode(&mut buf2) {
        return RoundtripResult::Error(anyhow!(error));
    }
    let buf2 = buf2;
    let buf3 = roundtrip.encode_to_vec();

    /*
    // Useful for debugging:
    eprintln!(" data: {:?}", data.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!(" buf1: {:?}", buf1.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!("a: {:?}\nb: {:?}", all_types, roundtrip);
    */

    if buf1 != buf2 {
        return RoundtripResult::Error(anyhow!("roundtripped encoded buffers do not match"));
    }

    if buf1 != buf3 {
        return RoundtripResult::Error(anyhow!(
            "roundtripped encoded buffers do not match with `encode_to_vec`"
        ));
    }

    RoundtripResult::Ok(buf1)
}

/// Generic roundtrip serialization check for messages.
pub fn check_message<M>(msg: &M)
where
    M: Message + Default + PartialEq,
{
    let expected_len = msg.encoded_len();

    let mut buf = Vec::with_capacity(18);
    msg.encode(&mut buf).unwrap();
    assert_eq!(expected_len, buf.len());

    let mut buf = buf.as_slice();
    let roundtrip = M::decode(&mut buf).unwrap();

    assert!(
        !buf.has_remaining(),
        "expected buffer to be empty: {}",
        buf.remaining()
    );
    assert_eq!(msg, &roundtrip);
}

/// Serialize from A should equal Serialize from B
pub fn check_serialize_equivalent<M, N>(msg_a: &M, msg_b: &N)
where
    M: Message + Default + PartialEq,
    N: Message + Default + PartialEq,
{
    let mut buf_a = Vec::new();
    msg_a.encode(&mut buf_a).unwrap();
    let mut buf_b = Vec::new();
    msg_b.encode(&mut buf_b).unwrap();
    assert_eq!(buf_a, buf_b);
}

#[cfg(test)]
mod tests {

    use alloc::collections::{BTreeMap, BTreeSet};
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::{boxed::Box, string::ToString};

    use super::*;

    use protobuf::test_messages::proto3::TestAllTypesProto3;

    #[test]
    fn test_all_types_proto3() {
        // Some selected encoded messages, mostly collected from failed fuzz runs.
        let msgs: &[&[u8]] = &[
            &[0x28, 0x28, 0x28, 0xFF, 0xFF, 0xFF, 0xFF, 0x68],
            &[0x92, 0x01, 0x00, 0x92, 0xF4, 0x01, 0x02, 0x00, 0x00],
            &[0x5d, 0xff, 0xff, 0xff, 0xff, 0x28, 0xff, 0xff, 0x21],
            &[0x98, 0x04, 0x02, 0x08, 0x0B, 0x98, 0x04, 0x02, 0x08, 0x02],
            // optional_int32: -1
            &[0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x08],
            // repeated_bool: [true, true]
            &[0xDA, 0x02, 0x02, 0x2A, 0x03],
            // oneof_double: nan
            &[0xb1, 0x07, 0xf6, 0x3d, 0xf5, 0xff, 0x27, 0x3d, 0xf5, 0xff],
            // optional_float: -0.0
            &[0xdd, 0x00, 0x00, 0x00, 0x00, 0x80],
            // optional_value: nan
            &[
                0xE2, 0x13, 0x1B, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
                0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0x08, 0xFF, 0x0E,
            ],
        ];

        for msg in msgs {
            roundtrip::<TestAllTypesProto3>(msg).unwrap();
        }
    }

    #[test]
    fn test_ident_conversions() {
        let msg = foo::bar_baz::FooBarBaz {
            foo_bar_baz: 42,
            fuzz_busters: vec![foo::bar_baz::foo_bar_baz::FuzzBuster {
                t: BTreeMap::<i32, foo::bar_baz::FooBarBaz>::new(),
                nested_self: None,
            }],
            p_i_e: foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::Foo.into(),
            r#as: 4,
            r#break: 5,
            r#const: 6,
            r#continue: 7,
            r#else: 8,
            r#enum: 9,
            r#false: 10,
            r#fn: 11,
            r#for: 12,
            r#if: 13,
            r#impl: 14,
            r#in: 15,
            r#let: 16,
            r#loop: 17,
            r#match: 18,
            r#mod: 19,
            r#move: 20,
            r#mut: 21,
            r#pub: 22,
            r#ref: 23,
            r#return: 24,
            r#static: 25,
            r#struct: 26,
            r#trait: 27,
            r#true: 28,
            r#type: 29,
            r#unsafe: 30,
            r#use: 31,
            r#where: 32,
            r#while: 33,
            r#dyn: 34,
            r#abstract: 35,
            r#become: 36,
            r#box: 37,
            r#do: 38,
            r#final: 39,
            r#macro: 40,
            r#override: 41,
            r#priv: 42,
            r#typeof: 43,
            r#unsized: 44,
            r#virtual: 45,
            r#yield: 46,
            r#async: 47,
            r#await: 48,
            r#try: 49,
            self_: 50,
            super_: 51,
            extern_: 52,
            crate_: 53,
        };

        let _ = foo::bar_baz::foo_bar_baz::Self_ {};

        // Test enum ident conversion.
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::Foo;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::Bar;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::FooBar;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::FuzzBuster;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::NormalRustEnumCase;

        let mut buf = Vec::new();
        msg.encode(&mut buf).expect("encode");
        roundtrip::<foo::bar_baz::FooBarBaz>(&buf).unwrap();
    }

    #[test]
    fn test_custom_type_attributes() {
        // We abuse the ident conversion protobuf for the custom attribute additions. We placed
        // `Ord` on the FooBarBaz (which is not implemented by ordinary messages).
        let mut set1 = BTreeSet::new();
        let msg1 = foo::bar_baz::FooBarBaz::default();
        set1.insert(msg1);
        // Similar, but for oneof fields
        let mut set2 = BTreeSet::new();
        let msg2 = oneof_attributes::Msg::default();
        set2.insert(msg2.field);
    }

    #[test]
    fn test_deep_nesting_oneof() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::recursive_oneof::{a, A, C};

            let mut a = Box::new(A {
                kind: Some(a::Kind::C(C {})),
            });
            for _ in 0..depth {
                a = Box::new(A {
                    kind: Some(a::Kind::A(a)),
                });
            }

            let mut buf = Vec::new();
            a.encode(&mut buf).unwrap();
            A::decode(buf.as_slice()).map(|_| ())
        }

        assert!(build_and_roundtrip(99).is_ok());
        assert!(build_and_roundtrip(100).is_err());
    }

    #[test]
    fn test_deep_nesting_group() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::groups::{nested_group2::OptionalGroup, NestedGroup2};

            let mut a = NestedGroup2::default();
            for _ in 0..depth {
                a = NestedGroup2 {
                    optionalgroup: Some(Box::new(OptionalGroup {
                        nested_group: Some(a),
                    })),
                };
            }

            let mut buf = Vec::new();
            a.encode(&mut buf).unwrap();
            NestedGroup2::decode(buf.as_slice()).map(|_| ())
        }

        assert!(build_and_roundtrip(50).is_ok());
        assert!(build_and_roundtrip(51).is_err());
    }

    #[test]
    fn test_recursive_oneof() {
        use crate::recursive_oneof::{a, A, B, C};
        let _ = A {
            kind: Some(a::Kind::B(Box::new(B {
                a: Some(Box::new(A {
                    kind: Some(a::Kind::C(C {})),
                })),
            }))),
        };
    }

    #[test]
    fn test_267_regression() {
        // Checks that skip_field will error appropriately when given a big stack of StartGroup
        // tags. When the no-recursion-limit feature is enabled this results in stack overflow.
        //
        // https://github.com/tokio-rs/prost/issues/267
        let buf = vec![b'C'; 1 << 20];
        <() as Message>::decode(&buf[..]).err().unwrap();
    }

    #[test]
    fn test_default_string_escape() {
        let msg = default_string_escape::Person::default();
        assert_eq!(msg.name, r#"["unknown"]"#);
    }

    #[test]
    fn test_group() {
        // optional group
        let msg1_bytes = &[0x0B, 0x10, 0x20, 0x0C];

        let msg1 = groups::Test1 {
            groupa: Some(groups::test1::GroupA { i2: Some(32) }),
        };

        let mut bytes = Vec::new();
        msg1.encode(&mut bytes).unwrap();
        assert_eq!(&bytes, msg1_bytes);

        // skip group while decoding
        let data: &[u8] = &[
            0x0B, // start group (tag=1)
            0x30, 0x01, // unused int32 (tag=6)
            0x2B, 0x30, 0xFF, 0x01, 0x2C, // unused group (tag=5)
            0x10, 0x20, // int32 (tag=2)
            0x0C, // end group (tag=1)
        ];
        assert_eq!(groups::Test1::decode(data), Ok(msg1));

        // repeated group
        let msg2_bytes: &[u8] = &[
            0x20, 0x40, 0x2B, 0x30, 0xFF, 0x01, 0x2C, 0x2B, 0x30, 0x01, 0x2C, 0x38, 0x64,
        ];

        let msg2 = groups::Test2 {
            i14: Some(64),
            groupb: vec![
                groups::test2::GroupB { i16: Some(255) },
                groups::test2::GroupB { i16: Some(1) },
            ],
            i17: Some(100),
        };

        let mut bytes = Vec::new();
        msg2.encode(&mut bytes).unwrap();
        assert_eq!(bytes.as_slice(), msg2_bytes);

        assert_eq!(groups::Test2::decode(msg2_bytes), Ok(msg2));
    }

    #[test]
    fn test_group_oneof() {
        let msg = groups::OneofGroup {
            i1: Some(42),
            field: Some(groups::oneof_group::Field::S2("foo".to_string())),
        };
        check_message(&msg);

        let msg = groups::OneofGroup {
            i1: Some(42),
            field: Some(groups::oneof_group::Field::G(groups::oneof_group::G {
                i2: None,
                s1: "foo".to_string(),
                t1: None,
            })),
        };
        check_message(&msg);

        let msg = groups::OneofGroup {
            i1: Some(42),
            field: Some(groups::oneof_group::Field::G(groups::oneof_group::G {
                i2: Some(99),
                s1: "foo".to_string(),
                t1: Some(groups::Test1 {
                    groupa: Some(groups::test1::GroupA { i2: None }),
                }),
            })),
        };
        check_message(&msg);

        check_message(&groups::OneofGroup::default());
    }

    #[test]
    fn test_proto3_presence() {
        let msg = proto3::presence::A {
            b: Some(42),
            foo: Some(proto3::presence::a::Foo::C(13)),
        };

        check_message(&msg);
    }

    #[test]
    fn test_file_descriptor_set_path() {
        let file_descriptor_set_bytes =
            include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));
        prost_types::FileDescriptorSet::decode(&file_descriptor_set_bytes[..]).unwrap();
    }
}
