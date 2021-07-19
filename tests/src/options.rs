use crate::check_message;
use prost::Message;
use std::fmt::{self, Debug};

#[derive(PartialEq, prost::Message)]
#[prost(debug = false)]
struct NoDebug {
    #[prost(int32, tag = "1")]
    foo: i32,
}

impl Debug for NoDebug {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("NoDebug")
           .field("foo", &self.foo)
           .finish()
    }
}

#[derive(PartialEq, prost::Message)]
#[prost(default = false)]
struct NoDefault {
    #[prost(int32, tag = "1")]
    foo: i32,
    bar: String,
}

impl Default for NoDefault {
    fn default() -> Self {
        NoDefault {
            foo: 0,
            bar: "bar".to_string(),
        }
    }
}

#[derive(PartialEq, prost::Message)]
#[prost(merge = false)]
struct NoMerge {
    #[prost(int32, tag = "1")]
    foo: i32,
    #[prost(message, required, tag = "2", to_msg = "|bar: &i32| *bar as u32")]
    bar: i32,
}

#[derive(PartialEq, prost::Message)]
struct NoClear {
    #[prost(int32, tag = "1")]
    foo: i32,
    #[prost(int32, tag = "2", clear = false)]
    bar: i32,
}

#[derive(PartialEq, prost::Message)]
#[prost(proto = "proto2")]
struct Proto2 {
    #[prost(message, tag = "1")]
    foo: Option<i32>,
}

#[derive(PartialEq, prost::Message)]
#[prost(proto = "proto3")]
struct Proto3 {
    #[prost(message, tag = "1")]
    foo: i32,
}

#[test]
fn no_debug() {
    let no_debug = NoDebug::default();
    check_message(&no_debug);

    assert_eq!(format!("{:?}", no_debug), "NoDebug { foo: 0 }");
}

#[test]
fn no_default() {
    let no_default = NoDefault::default();
    check_message(&no_default);

    assert_eq!(format!("{:?}", no_default), "NoDefault { foo: 0 }");
}

#[test]
fn no_merge() {
    let no_merge = NoMerge::default();

    let mut buf = Vec::with_capacity(no_merge.encoded_len());
    no_merge.encode(&mut buf).expect("failed encoding");

    assert!(NoMerge::decode(buf.as_ref()).is_err());
}

#[test]
fn no_clear() {
    let mut no_clear = NoClear {
        foo: 42,
        bar: 42,
    };

    check_message(&no_clear);
    no_clear.clear();

    assert_eq!(no_clear.foo, 0);
    assert_eq!(no_clear.bar, 42);
}
