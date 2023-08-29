pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

#[cfg(test)]
mod test {
    use super::hello_world::new_hello_world::*;
    use super::hello_world::*;
    use prost::Message;

    #[test]
    fn test_hello() {
        let mut msg = NewHelloWorld::default();
        msg.foo = 32;
        msg.new_field = 45;
        msg.bar = Some(Bar::Inner1(Inner {
            cool_string: "really cool".to_string(),
            ..Inner::default()
        }));
        let mut buf = Vec::new();
        msg.encode_raw(&mut buf);

        let decoded = OldHelloWorld::decode(buf.as_slice()).unwrap();
        println!("{:?}", decoded);
        let mut buf = Vec::new();
        decoded.encode_raw(&mut buf);

        let decoded = NewHelloWorld::decode(buf.as_slice()).unwrap();
        assert_eq!(msg, decoded);
    }
}
