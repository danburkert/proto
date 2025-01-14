use alloc::boxed::Box;
use alloc::vec::Vec;
use prost::Message;

include!(concat!(env!("OUT_DIR"), "/recursive_oneof.rs"));

#[test]
fn test_recursive_oneof() {
    let _ = A {
        kind: Some(a::Kind::B(Box::new(B {
            a: Some(Box::new(A {
                kind: Some(a::Kind::C(C {})),
            })),
        }))),
    };
}

#[test]
fn test_deep_nesting_oneof() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
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
