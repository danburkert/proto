use core::fmt::Debug;
use heapless::{String, Vec};
use prost::Message;

// Helper for encode/decode tests
fn encode_decode<T: Debug + PartialEq + Default + Message>(a: T) {
    let buff = a.encode_to_vec();

    let b = T::decode(&*buff).unwrap();

    assert_eq!(a, b);
}

/// [`heapless::String`] container
#[derive(Clone, PartialEq, Message)]
pub struct TestString {
    #[prost(message, required, tag = "1")]
    pub s: String<32>,
}

#[test]
fn encode_decode_string() {
    encode_decode(TestString {
        s: String::from("abc1234"),
    });
}

/// [`heapless::Vec`] container
#[derive(Clone, PartialEq, Message)]
pub struct TestVec {
    #[prost(message, required, tag = "1")]
    pub s: Vec<u8, 32>,
}

#[test]
fn encode_decode_bytes() {
    encode_decode(TestVec {
        s: Vec::from_slice(&[0xaa, 0xbb, 0xcc]).unwrap(),
    });
}
