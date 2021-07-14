use afl::fuzz;

use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

fn main() {
    fuzz!(|data: &[u8]| {
        let _ = roundtrip::<TestAllTypesProto3>(data).unwrap_error();
    });
}
