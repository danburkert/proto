# proto3 fuzz tests

## Test corpus

The test message `testmessage` was created like this:

```rust
use prost::Message;
use protobuf::test_messages::proto3::TestAllTypesProto3;

fn main() {
    let msg = TestAllTypesProto3 {
        optional_int32: 42,
        optional_fixed64: 9983748923,
        optional_bool: true,
        recursive_message: Some(
            Box::new(TestAllTypesProto3 {
                repeated_int32: vec![1, 2, 99, 50, -5],
                ..Default::default()
            })
        ),
        repeated_sfixed32: vec![1, -1, 1, -1],
        repeated_float: vec![-1.0, 10.10, 1.337, std::f32::NAN],
        ..Default::default()
    };
    let mut buf = vec![];
    msg.encode(&mut buf).unwrap();
    std::fs::write("proto3-default.bin", buf).unwrap();
}
```
