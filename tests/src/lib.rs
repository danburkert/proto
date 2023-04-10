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
        extern crate bytes;
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
mod generic_derive;
#[cfg(test)]
mod message_encoding;
#[cfg(test)]
mod no_unused_results;
#[cfg(test)]
mod well_known_types;

pub mod foo {
    pub mod bar_baz {
        include!(concat!(env!("OUT_DIR"), "/foo.bar_baz.rs"));
    }
}

pub mod nesting {
    include!(concat!(env!("OUT_DIR"), "/nesting.rs"));
}

pub mod recursive_oneof {
    include!(concat!(env!("OUT_DIR"), "/recursive_oneof.rs"));
}

/// This tests the custom attributes support by abusing docs.
///
/// Docs really are full-blown attributes. So we use them to ensure we can place them on everything
/// we need. If they aren't put onto something or allowed not to be there (by the generator),
/// compilation fails.
#[deny(missing_docs)]
pub mod custom_attributes {
    include!(concat!(env!("OUT_DIR"), "/foo.custom.attrs.rs"));
}

/// Also for testing custom attributes, but on oneofs.
///
/// Unfortunately, an OneOf field generates a companion module in the .rs file. There's no
/// reasonable way to place a doc comment on that, so we do the test with `derive(Ord)` and have it
/// in a separate file.
pub mod oneof_attributes {
    include!(concat!(env!("OUT_DIR"), "/foo.custom.one_of_attrs.rs"));
}

/// Issue https://github.com/tokio-rs/prost/issues/118
///
/// When a message contains an enum field with a default value, we
/// must ensure that the appropriate name conventions are used.
pub mod default_enum_value {
    include!(concat!(env!("OUT_DIR"), "/default_enum_value.rs"));
}

pub mod groups {
    include!(concat!(env!("OUT_DIR"), "/groups.rs"));
}

pub mod proto3 {
    pub mod presence {
        include!(concat!(env!("OUT_DIR"), "/proto3.presence.rs"));
    }
}

pub mod invalid {
    pub mod doctest {
        include!(concat!(env!("OUT_DIR"), "/invalid.doctest.rs"));
    }
}

pub mod default_string_escape {
    include!(concat!(env!("OUT_DIR"), "/default_string_escape.rs"));
}

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
        return RoundtripResult::Error(error.into());
    }
    if encoded_len != buf1.len() {
        return RoundtripResult::Error(anyhow!(
            "expected encoded len ({}) did not match actual encoded len ({})",
            encoded_len,
            buf1.len()
        ));
    }

    let roundtrip = match M::decode(&*buf1) {
        Ok(roundtrip) => roundtrip,
        Err(error) => return RoundtripResult::Error(anyhow::Error::new(error)),
    };

    let mut buf2 = Vec::new();
    if let Err(error) = roundtrip.encode(&mut buf2) {
        return RoundtripResult::Error(error.into());
    }
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

/// Generic rountrip serialization check for messages.
pub fn check_message<M>(msg: &M)
where
    M: Message + Default + PartialEq,
{
    let expected_len = msg.encoded_len();

    let mut buf = Vec::with_capacity(18);
    msg.encode(&mut buf).unwrap();
    assert_eq!(expected_len, buf.len());

    let mut buf = &*buf;
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

    use alloc::borrow::ToOwned;
    use alloc::boxed::Box;
    use alloc::collections::{BTreeMap, BTreeSet};
    use alloc::string::ToString;
    use alloc::vec;

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
            p_i_e: 0,
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
    fn test_nesting() {
        use crate::nesting::{A, B};
        let _ = A {
            a: Some(Box::new(A::default())),
            repeated_a: Vec::<A>::new(),
            map_a: BTreeMap::<i32, A>::new(),
            b: Some(Box::new(B::default())),
            repeated_b: Vec::<B>::new(),
            map_b: BTreeMap::<i32, B>::new(),
        };
    }

    #[test]
    fn test_deep_nesting() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::nesting::A;

            let mut a = Box::new(A::default());
            for _ in 0..depth {
                let mut next = Box::new(A::default());
                next.a = Some(a);
                a = next;
            }

            let mut buf = Vec::new();
            a.encode(&mut buf).unwrap();
            A::decode(&*buf).map(|_| ())
        }

        assert!(build_and_roundtrip(100).is_ok());
        assert!(build_and_roundtrip(101).is_err());
    }

    #[test]
    fn test_deep_nesting_with_custom_recursion_limit() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::nesting::E;

            let mut e = Box::new(E::default());
            for _ in 0..depth {
                let mut next = Box::new(E::default());
                next.e = Some(e);
                e = next;
            }

            let mut buf = Vec::new();
            e.encode(&mut buf).unwrap();
            E::decode(&*buf).map(|_| ())
        }

        assert!(build_and_roundtrip(200).is_ok());
        assert!(build_and_roundtrip(201).is_err());
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
            A::decode(&*buf).map(|_| ())
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
            NestedGroup2::decode(&*buf).map(|_| ())
        }

        assert!(build_and_roundtrip(50).is_ok());
        assert!(build_and_roundtrip(51).is_err());
    }

    #[test]
    fn test_deep_nesting_repeated() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::nesting::C;

            let mut c = C::default();
            for _ in 0..depth {
                let mut next = C::default();
                next.r.push(c);
                c = next;
            }

            let mut buf = Vec::new();
            c.encode(&mut buf).unwrap();
            C::decode(&*buf).map(|_| ())
        }

        assert!(build_and_roundtrip(100).is_ok());
        assert!(build_and_roundtrip(101).is_err());
    }

    #[test]
    fn test_deep_nesting_map() {
        fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
            use crate::nesting::D;

            let mut d = D::default();
            for _ in 0..depth {
                let mut next = D::default();
                next.m.insert("foo".to_owned(), d);
                d = next;
            }

            let mut buf = Vec::new();
            d.encode(&mut buf).unwrap();
            D::decode(&*buf).map(|_| ())
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
    fn test_default_enum() {
        let msg = default_enum_value::Test::default();
        assert_eq!(msg.privacy_level_1(), default_enum_value::PrivacyLevel::One);
        assert_eq!(
            msg.privacy_level_3(),
            default_enum_value::PrivacyLevel::PrivacyLevelThree
        );
        assert_eq!(
            msg.privacy_level_4(),
            default_enum_value::PrivacyLevel::PrivacyLevelprivacyLevelFour
        );
    }

    #[test]
    fn test_enum_to_string() {
        use default_enum_value::{ERemoteClientBroadcastMsg, PrivacyLevel};

        assert_eq!(PrivacyLevel::One.as_str_name(), "PRIVACY_LEVEL_ONE");
        assert_eq!(PrivacyLevel::Two.as_str_name(), "PRIVACY_LEVEL_TWO");
        assert_eq!(
            PrivacyLevel::PrivacyLevelThree.as_str_name(),
            "PRIVACY_LEVEL_PRIVACY_LEVEL_THREE"
        );
        assert_eq!(
            PrivacyLevel::PrivacyLevelprivacyLevelFour.as_str_name(),
            "PRIVACY_LEVELPRIVACY_LEVEL_FOUR"
        );

        assert_eq!(
            ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery.as_str_name(),
            "k_ERemoteClientBroadcastMsgDiscovery"
        );
    }

    #[test]
    fn test_enum_from_string() {
        use default_enum_value::{ERemoteClientBroadcastMsg, PrivacyLevel};

        assert_eq!(
            Some(PrivacyLevel::One),
            PrivacyLevel::from_str_name("PRIVACY_LEVEL_ONE")
        );
        assert_eq!(
            Some(PrivacyLevel::Two),
            PrivacyLevel::from_str_name("PRIVACY_LEVEL_TWO")
        );
        assert_eq!(
            Some(PrivacyLevel::PrivacyLevelThree),
            PrivacyLevel::from_str_name("PRIVACY_LEVEL_PRIVACY_LEVEL_THREE")
        );
        assert_eq!(
            Some(PrivacyLevel::PrivacyLevelprivacyLevelFour),
            PrivacyLevel::from_str_name("PRIVACY_LEVELPRIVACY_LEVEL_FOUR")
        );
        assert_eq!(None, PrivacyLevel::from_str_name("PRIVACY_LEVEL_FIVE"));

        assert_eq!(
            Some(ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery),
            ERemoteClientBroadcastMsg::from_str_name("k_ERemoteClientBroadcastMsgDiscovery")
        );
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
        assert_eq!(&*bytes, msg2_bytes);

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
