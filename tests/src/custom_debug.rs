//! Tests for skipping the default Debug implementation.

include!(concat!(env!("OUT_DIR"), "/custom_debug.rs"));

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::fmt;
use prost::OpenEnum;

impl fmt::Debug for Msg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Msg {..}")
    }
}

macro_rules! enum_tests {
    () => {
        impl fmt::Debug for NewType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("NewType(custom_debug)")
            }
        }

        /// A special case with a tuple struct
        #[test]
        fn tuple_struct_custom_debug() {
            assert_eq!(
                format!("{:?}", NewType(AnEnum::B.into())),
                "NewType(custom_debug)"
            );
        }

        impl fmt::Debug for OneofWithEnumCustomDebug {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("OneofWithEnumCustomDebug {..}")
            }
        }

        #[derive(Clone, PartialEq, prost::Message)]
        #[prost(skip_debug)]
        struct MessageWithOneofCustomDebug {
            #[prost(oneof = "OneofWithEnumCustomDebug", tags = "8, 9, 10")]
            of: Option<OneofWithEnumCustomDebug>,
        }

        impl fmt::Debug for MessageWithOneofCustomDebug {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("MessageWithOneofCustomDebug {..}")
            }
        }

        /// Enumerations inside oneofs
        #[test]
        fn oneof_with_enum_custom_debug() {
            let of = OneofWithEnumCustomDebug::Enumeration(AnEnum::B.into());
            assert_eq!(format!("{:?}", of), "OneofWithEnumCustomDebug {..}");
            let msg = MessageWithOneofCustomDebug { of: Some(of) };
            assert_eq!(format!("{:?}", msg), "MessageWithOneofCustomDebug {..}");
        }
    };
}

mod int_enum {
    use super::*;

    #[derive(Clone, PartialEq, prost::Message)]
    #[prost(skip_debug)]
    struct NewType(#[prost(enumeration = "AnEnum", tag = "5")] i32);

    #[derive(Clone, PartialEq, prost::Oneof)]
    #[prost(skip_debug)]
    pub enum OneofWithEnumCustomDebug {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "AnEnum", tag = "10")]
        Enumeration(i32),
    }

    enum_tests!();
}

mod open_enum {
    use super::*;

    #[derive(Clone, PartialEq, prost::Message)]
    #[prost(skip_debug)]
    struct NewType(
        #[prost(enumeration = "AnEnum", enum_type = "open", tag = "5")] OpenEnum<AnEnum>,
    );

    #[derive(Clone, PartialEq, prost::Oneof)]
    #[prost(skip_debug)]
    pub enum OneofWithEnumCustomDebug {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "AnEnum", enum_type = "open", tag = "10")]
        Enumeration(OpenEnum<AnEnum>),
    }

    enum_tests!();
}

mod closed_enum {
    use super::*;

    #[derive(Clone, PartialEq, prost::Message)]
    #[prost(skip_debug)]
    struct NewType(#[prost(enumeration = "AnEnum", enum_type = "closed", tag = "5")] AnEnum);

    #[derive(Clone, PartialEq, prost::Oneof)]
    #[prost(skip_debug)]
    pub enum OneofWithEnumCustomDebug {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "AnEnum", enum_type = "closed", tag = "10")]
        Enumeration(AnEnum),
    }

    enum_tests!();
}

/// Generated protobufs
#[test]
fn test_proto_msg_custom_debug() {
    let msg = Msg {
        a: 0,
        b: "".to_string(),
        c: Some(msg::C::D(AnEnum::A.into())),
    };
    assert_eq!(format!("{:?}", msg), "Msg {..}");
}
