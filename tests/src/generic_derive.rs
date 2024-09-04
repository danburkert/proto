pub trait CustomType: prost::Message + Default + core::fmt::Debug {}

impl CustomType for u64 {}

#[derive(Clone, prost::Oneof)]
enum GenericEnum<A: CustomType> {
    #[prost(message, tag = "1")]
    Data(GenericMessage<A>),
    #[prost(uint64, tag = "2")]
    #[allow(dead_code)]
    Number(u64),
}

#[derive(Clone, prost::Message)]
struct GenericMessage<A: CustomType> {
    #[prost(message, tag = "1")]
    data: Option<A>,
}

#[test]
fn generic_enum() {
    let msg = GenericMessage { data: Some(100u64) };
    let enumeration = GenericEnum::Data(msg);
    match enumeration {
        GenericEnum::Data(d) => assert_eq!(100, d.data.unwrap()),
        GenericEnum::Number(_) => panic!("Not supposed to reach"),
    }
}
