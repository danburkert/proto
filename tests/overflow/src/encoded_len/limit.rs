//! Test that the limits returned by field::scalar::Ty::encoded_len_limit
//! can be reached with actual values.

macro_rules! test_limit {
    ($name:ident, $value:expr, $limit:expr) => {
        mod $name {
            use crate::encoded_len::proto;
            use prost::Message;

            #[test]
            fn encoded_len_limit_is_reachable() {
                let msg = proto::Testbed {
                    $name: $value,
                    ..Default::default()
                };
                assert_eq!(msg.encoded_len() - 1, $limit);
            }
        }
    };
}

test_limit!(int32, -1, 10);
test_limit!(int64, -1, 10);
test_limit!(uint32, u32::MAX, 5);
test_limit!(uint64, u64::MAX, 10);
test_limit!(sint32, i32::MIN, 5);
test_limit!(sint64, i64::MIN, 10);
test_limit!(fixed32, 1, 4);
test_limit!(fixed64, 1, 8);
test_limit!(sfixed32, -1, 4);
test_limit!(sfixed64, -1, 8);
test_limit!(float, 1.0, 4);
test_limit!(double, 1.0, 8);
test_limit!(bool, true, 1);
test_limit!(enumeration, proto::BadEnum::Long as i32, 10);
