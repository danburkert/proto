//! Tests for our own Debug implementation.
//!
//! The tests check against expected output. This may be a bit fragile, but it is likely OK for
//! actual use.

use prost::alloc::format;
#[cfg(not(feature = "std"))]
use prost::alloc::string::String;

use crate::message_encoding::BasicEnumeration;

#[cfg(feature = "std")]
const BASIC_DEFAULT_OUTPUT: &str = "Basic { \
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
}";

#[cfg(feature = "std")]
const BASIC_KNOWN_OUTPUT: &str = "Basic { \
    int32: 0, \
    bools: [], \
    string: \"\", \
    optional_string: None, \
    enumeration: ONE, \
    enumeration_map: {0: TWO}, \
    string_map: {}, \
    enumeration_btree_map: {}, \
    string_btree_map: {}, \
    oneof: None, \
    bytes_map: {\"hello\": [119, 111, 114, 108, 100]} \
}";

macro_rules! message_with_oneof {
    () => {
        #[derive(Clone, PartialEq, prost::Message)]
        struct MessageWithOneof {
            #[prost(oneof = "OneofWithEnum", tags = "8, 9, 10")]
            of: Option<OneofWithEnum>,
        }
    };
}

macro_rules! known_enum_tests {
    () => {
        /// Some real-life message
        #[test]
        #[cfg(feature = "std")]
        fn basic_known() {
            let mut basic = Basic::default();
            assert_eq!(format!("{:?}", basic), BASIC_DEFAULT_OUTPUT);
            basic
                .enumeration_map
                .insert(0, BasicEnumeration::TWO.into());
            basic.enumeration = BasicEnumeration::ONE.into();
            basic
                .bytes_map
                .insert("hello".to_string(), "world".as_bytes().into());
            assert_eq!(format!("{:?}", basic), BASIC_KNOWN_OUTPUT);
        }

        /// A special case with a tuple struct
        #[test]
        fn tuple_struct_known() {
            assert_eq!(
                format!("{:?}", NewType(BasicEnumeration::TWO.into())),
                "NewType(TWO)"
            );
        }

        /// Enumerations inside oneofs
        #[test]
        fn oneof_with_enum_known() {
            let msg = MessageWithOneof {
                of: Some(OneofWithEnum::Enumeration(BasicEnumeration::TWO.into())),
            };
            assert_eq!(
                format!("{:?}", msg),
                "MessageWithOneof { of: Some(Enumeration(TWO)) }"
            );
        }
    };
}

macro_rules! unknown_enum_tests {
    ($from_raw:expr) => {
        /// Some real-life message
        #[test]
        #[cfg(feature = "std")]
        fn basic_unknown() {
            let mut basic = Basic::default();
            assert_eq!(format!("{:?}", basic), BASIC_DEFAULT_OUTPUT);
            basic.enumeration_map.insert(0, $from_raw(42));
            basic.enumeration = $from_raw(42);
            assert_eq!(format!("{:?}", basic), BASIC_UNKNOWN_OUTPUT);
        }

        /// A special case with a tuple struct
        #[test]
        fn tuple_struct_unknown() {
            assert_eq!(
                format!("{:?}", NewType($from_raw(42))),
                NEWTYPE_UNKNOWN_OUTPUT,
            );
        }

        /// Enumerations inside oneofs
        #[test]
        fn oneof_with_enum_unknown() {
            let msg = MessageWithOneof {
                of: Some(OneofWithEnum::Enumeration($from_raw(42))),
            };
            assert_eq!(
                format!("{:?}", msg),
                format!(
                    "MessageWithOneof {{ of: Some({}) }}",
                    ONEOF_ENUMERATION_UNKNOWN_OUTPUT
                ),
            );
        }
    };
}

mod int_enum {
    use super::*;

    use core::convert::identity;

    // Borrow some types from other places.
    #[cfg(feature = "std")]
    use crate::message_encoding::int_enum::Basic;

    #[cfg(feature = "std")]
    const BASIC_UNKNOWN_OUTPUT: &str = "Basic { \
        int32: 0, \
        bools: [], \
        string: \"\", \
        optional_string: None, \
        enumeration: 42, \
        enumeration_map: {0: 42}, \
        string_map: {}, \
        enumeration_btree_map: {}, \
        string_btree_map: {}, \
        oneof: None, \
        bytes_map: {} \
    }";

    const NEWTYPE_UNKNOWN_OUTPUT: &str = "NewType(42)";

    const ONEOF_ENUMERATION_UNKNOWN_OUTPUT: &str = "Enumeration(42)";

    #[derive(Clone, PartialEq, prost::Message)]
    struct NewType(#[prost(enumeration = "BasicEnumeration", tag = "5")] i32);

    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum OneofWithEnum {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "BasicEnumeration", tag = "10")]
        Enumeration(i32),
    }

    message_with_oneof!();
    known_enum_tests!();
    unknown_enum_tests!(identity);
}

mod open_enum {
    use super::*;

    use prost::OpenEnum;

    // Borrow some types from other places.
    #[cfg(feature = "std")]
    use crate::message_encoding::open_enum::Basic;

    #[cfg(feature = "std")]
    const BASIC_UNKNOWN_OUTPUT: &str = "Basic { \
        int32: 0, \
        bools: [], \
        string: \"\", \
        optional_string: None, \
        enumeration: Unknown(42), \
        enumeration_map: {0: Unknown(42)}, \
        string_map: {}, \
        enumeration_btree_map: {}, \
        string_btree_map: {}, \
        oneof: None, \
        bytes_map: {} \
    }";

    const NEWTYPE_UNKNOWN_OUTPUT: &str = "NewType(Unknown(42))";

    const ONEOF_ENUMERATION_UNKNOWN_OUTPUT: &str = "Enumeration(Unknown(42))";

    #[derive(Clone, PartialEq, prost::Message)]
    struct NewType(
        #[prost(enumeration = "BasicEnumeration", enum_type = "open", tag = "5")]
        OpenEnum<BasicEnumeration>,
    );

    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum OneofWithEnum {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "BasicEnumeration", enum_type = "open", tag = "10")]
        Enumeration(OpenEnum<BasicEnumeration>),
    }

    message_with_oneof!();
    known_enum_tests!();
    unknown_enum_tests!(OpenEnum::from_raw);
}

mod closed_enum {
    use super::*;

    // Borrow some types from other places.
    #[cfg(feature = "std")]
    use crate::message_encoding::closed_enum::Basic;

    #[derive(Clone, PartialEq, prost::Message)]
    struct NewType(
        #[prost(enumeration = "BasicEnumeration", enum_type = "closed", tag = "5")]
        BasicEnumeration,
    );

    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum OneofWithEnum {
        #[prost(int32, tag = "8")]
        Int(i32),
        #[prost(string, tag = "9")]
        String(String),
        #[prost(enumeration = "BasicEnumeration", enum_type = "closed", tag = "10")]
        Enumeration(BasicEnumeration),
    }

    message_with_oneof!();
    known_enum_tests!();
}
