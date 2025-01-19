//! Tests for integer overflow behavior of encoded_len* functions.
//!
//! Many of the tests in this module allocate and fill large amounts of RAM,
//! so they should better be run in isolation and in a sufficiently
//! memory-budgeted environment. Particularly on 32-bit platforms, the tests
//! should not be run in parallel as the combined allocation requests can exceed
//! the addressable memory.

use crate::encoded_len::proto;

use prost::alloc::vec;

#[cfg(target_pointer_width = "64")]
fn verify_overflowing_encoded_len(actual: usize, expected: u64) -> bool {
    if actual as u64 == expected {
        true
    } else {
        cfg_if! {
            if #[cfg(feature = "std")] {
                eprintln!("expected {} but the function returned {}", expected, actual);
            }
        }
        false
    }
}

#[cfg(target_pointer_width = "32")]
fn verify_overflowing_encoded_len(actual: usize, _expected: u64) -> bool {
    // Tests calling this function are expected to panic on 32-bit platforms
    // before this check is called. Returning true here allows the
    // #[should_panic] tests to fail.
    cfg_if! {
        if #[cfg(feature = "std")] {
            eprintln!("expected panic, but the function returned {actual}");
        }
    }
    true
}

mod field {
    // Test encoded_len* functions in prost::encoding submodules for various field types.

    use super::*;

    mod bool {
        use super::*;
        use prost::encoding::{bool, MAX_TAG};

        #[test]
        #[ignore = "allocates and fills about 666 MiB"]
        #[cfg_attr(target_pointer_width = "32", should_panic)]
        fn encoded_len_repeated_can_overflow_u32() {
            let filler = false;
            let filler_len = bool::encoded_len(MAX_TAG, &filler);
            let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
            let encoded_len = bool::encoded_len_repeated(MAX_TAG, &bomb32);
            let expected_len = bomb32.len() as u64 * filler_len as u64;
            assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
        }
    }

    // These tests may abort on the large allocations when built on a 32-bit
    // target and run in company with some other tests, even with
    // --test-threads=1.
    // As heap fragmentation seemingly becomes a problem, these tests are best
    // run in isolation.
    mod int32 {
        use super::*;
        use prost::encoding::{int32, MAX_TAG};

        #[test]
        #[ignore = "allocates and fills more than 1 GiB"]
        #[cfg_attr(target_pointer_width = "32", should_panic)]
        fn encoded_len_repeated_can_overflow_u32() {
            let filler = -1i32;
            let filler_len = int32::encoded_len(MAX_TAG, &filler);
            let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
            let encoded_len = int32::encoded_len_repeated(MAX_TAG, &bomb32);
            let expected_len = bomb32.len() as u64 * filler_len as u64;
            assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
        }

        #[test]
        #[ignore = "allocates and fills about 1.6 GiB"]
        #[cfg_attr(target_pointer_width = "32", should_panic)]
        fn encoded_len_packed_can_overflow_u32() {
            use prost::encoding::{encoded_len_varint, key_len};

            let filler = -1i32;
            let filler_len = encoded_len_varint(filler as u64);
            let bomb_len = (u32::MAX as usize - key_len(MAX_TAG) - 5) / filler_len + 1;
            let bomb32 = vec![filler; bomb_len];
            let encoded_len = int32::encoded_len_packed(MAX_TAG, &bomb32);
            let expected_data_len = bomb_len as u64 * filler_len as u64;
            let expected_len = key_len(MAX_TAG) as u64
                + encoded_len_varint(expected_data_len) as u64
                + expected_data_len;
            assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
        }
    }

    mod message {
        use super::*;
        use crate::encoded_len::proto;
        use prost::encoding::{encoded_len_varint, key_len, message, MAX_TAG};

        #[test]
        #[cfg_attr(target_pointer_width = "32", should_panic)]
        fn encoded_len_can_overflow_u32() {
            let filler = proto::Empty {};
            let filler_len = message::encoded_len(MAX_TAG, &filler);
            let subcritical = vec![filler; u32::MAX as usize / filler_len];
            let payload_len = subcritical.len() * filler_len;
            assert_eq!(encoded_len_varint(payload_len as u64), 5);
            assert!(key_len(MAX_TAG) + 5 >= filler_len);
            let bomb32 = proto::Testbed {
                repeated_empty: subcritical,
                ..Default::default()
            };
            let encoded_len = message::encoded_len(MAX_TAG, &bomb32);
            let expected_len = (key_len(MAX_TAG) + 5) as u64 + payload_len as u64;
            assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
        }

        #[test]
        #[cfg_attr(target_pointer_width = "32", should_panic)]
        fn encoded_len_repeated_can_overflow_u32() {
            let filler = proto::Empty {};
            let filler_len = message::encoded_len(MAX_TAG, &filler);
            let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
            let encoded_len = message::encoded_len_repeated(MAX_TAG, &bomb32);
            let expected_len = bomb32.len() as u64 * filler_len as u64;
            assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
        }
    }

    macro_rules! test_map {
        ($map_mod:ident, $map_init:expr) => {
            use prost::encoding::{int32, message, $map_mod, MAX_TAG};

            #[test]
            #[ignore = "allocates and fills about 1 GiB"]
            #[cfg_attr(target_pointer_width = "32", should_panic)]
            fn encoded_len_can_overflow_u32() {
                let encoded_entry_len = 5 + 1 + 1 + 10;
                let num_entries = u32::MAX as usize / encoded_entry_len + 1;
                let mut map = $map_init(num_entries);
                map.extend((-(num_entries as i32)..0).map(|i| (i, proto::Empty {})));
                let encoded_len =
                    $map_mod::encoded_len(int32::encoded_len, message::encoded_len, MAX_TAG, &map);
                let expected_len = num_entries as u64 * encoded_entry_len as u64;
                assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
            }
        };
    }

    mod btree_map {
        use super::*;
        use prost::alloc::collections::BTreeMap;

        test_map!(btree_map, |_| BTreeMap::new());
    }

    #[cfg(feature = "std")]
    mod hash_map {
        use super::*;
        use std::collections::HashMap;

        test_map!(hash_map, HashMap::with_capacity);
    }
}

mod derived {
    // Test overflow behavior of Message::encoded_len method implementations
    // generated by prost-derive.

    use super::*;
    use crate::encoded_len::proto;
    use prost::alloc::collections::BTreeMap;
    use prost::alloc::string::String;
    use prost::alloc::vec::Vec;
    use prost::encoding::{message, MAX_TAG};
    use prost::Message;

    // Initializes all scalar fields so as to give the largest possible
    // encodings for these fields.
    const FATTEST_SCALARS: proto::Testbed = proto::Testbed {
        int32: -1,
        int64: -1,
        uint32: u32::MAX,
        uint64: u64::MAX,
        sint32: i32::MIN,
        sint64: i64::MIN,
        fixed32: 1,
        fixed64: 1,
        sfixed32: -1,
        sfixed64: -1,
        float: 1.0,
        double: 1.0,
        bool: true,
        enumeration: proto::BadEnum::Long as i32,
        string: String::new(),
        bytes: Vec::new(),
        packed_int32: vec![],
        map: BTreeMap::new(),
        repeated_empty: vec![],
    };

    const SCALAR_ENCODED_LEN_LIMITS: &[usize] = &[
        10, // int32
        10, // int64
        5,  // uint32
        10, // uint64
        5,  // sint32
        10, // sint64
        4,  // fixed32
        8,  // fixed64
        4,  // sfixed32
        8,  // sfixed64
        4,  // float
        8,  // double
        1,  // bool
        10, // enumeration
    ];

    #[test]
    fn limited_length_scalar_encodings_are_accounted_for() {
        assert_eq!(
            FATTEST_SCALARS.encoded_len(),
            SCALAR_ENCODED_LEN_LIMITS
                .iter()
                .cloned()
                .map(|len| 1 + len)
                .sum()
        );
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_with_repeated_field() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let supercritical =
            vec![filler; (u32::MAX as usize - FATTEST_SCALARS.encoded_len()) / filler_len + 1];
        let payload_len = supercritical.len() as u64 * filler_len as u64;
        let bomb32 = proto::Testbed {
            repeated_empty: supercritical,
            ..FATTEST_SCALARS
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len = FATTEST_SCALARS.encoded_len() as u64 + payload_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_all_checked_with_repeated_field() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let supercritical = vec![filler; u32::MAX as usize / filler_len + 1];
        let expected_len = supercritical.len() as u64 * filler_len as u64;
        let bomb32 = proto::TwoUnlimited {
            repeated_empty: supercritical,
            ..Default::default()
        };
        let encoded_len = bomb32.encoded_len();
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_with_string() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let padding =
            vec![filler; (u32::MAX as usize - FATTEST_SCALARS.encoded_len()) / filler_len];
        let padding_len = padding.len() * filler_len;
        let bomb32 = proto::Testbed {
            repeated_empty: padding,
            string: " ".repeat(filler_len - 2 - 1),
            ..FATTEST_SCALARS
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len =
            FATTEST_SCALARS.encoded_len() as u64 + padding_len as u64 + filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_all_checked_with_string() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let padding = vec![filler; u32::MAX as usize / filler_len];
        let padding_len = padding.len() * filler_len;
        let bomb32 = proto::TwoUnlimited {
            repeated_empty: padding,
            string: " ".repeat(filler_len - 2 - 1),
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len = padding_len as u64 + filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_with_bytes() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let padding =
            vec![filler; (u32::MAX as usize - FATTEST_SCALARS.encoded_len()) / filler_len];
        let padding_len = padding.len() * filler_len;
        let bomb32 = proto::Testbed {
            repeated_empty: padding,
            bytes: b" ".repeat(filler_len - 2 - 1),
            ..FATTEST_SCALARS
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len =
            FATTEST_SCALARS.encoded_len() as u64 + padding_len as u64 + filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_with_packed_varint() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let padding =
            vec![filler; (u32::MAX as usize - FATTEST_SCALARS.encoded_len()) / filler_len];
        let padding_len = padding.len() * filler_len;
        let bomb32 = proto::Testbed {
            repeated_empty: padding,
            packed_int32: vec![0; filler_len - 2 - 1],
            ..FATTEST_SCALARS
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len =
            FATTEST_SCALARS.encoded_len() as u64 + padding_len as u64 + filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32_with_map() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let padding =
            vec![filler; (u32::MAX as usize - FATTEST_SCALARS.encoded_len()) / filler_len];
        let padding_len = padding.len() * filler_len;
        let map = [(0, -1)].iter().cloned().collect();
        let bomb32 = proto::Testbed {
            repeated_empty: padding,
            map,
            ..FATTEST_SCALARS
        };
        let encoded_len = bomb32.encoded_len();
        let expected_len = FATTEST_SCALARS.encoded_len() as u64 + padding_len as u64 + 14;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }
}
