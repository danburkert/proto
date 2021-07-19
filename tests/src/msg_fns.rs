use prost::Message;
use std::collections::BTreeMap;

#[derive(PartialEq, prost::Message)]
struct WithMsgFns {
    #[prost(
        uint32,
        tag = "1",
        as_msg = "get_tuple_left",
        merge_msg = "|t: &mut (u32, String), l: u32| t.0 = l"
    )]
    #[prost(
        string,
        tag = "2",
        as_msg = "get_tuple_right",
        merge_msg = "|t: &mut (u32, String), r: String| t.1 = r"
    )]
    tuple: (u32, String),
    #[prost(
        uint32,
        tag = "3",
        to_msg = "|n: &i32| n.abs() as u32",
        from_msg = "|p: u32| (p as i32) * -1"
    )]
    neg_to_pos: i32,
    #[prost(
        uint32,
        optional,
        tag = "4",
        to_msg = "|_: &Option<u32>| Option::<u32>::None",
        from_msg = "|a: Option<u32>| a"
    )]
    none: Option<u32>,
    #[prost(
        uint32,
        tag = "5",
        as_msg = "get_msg_field",
        from_msg = "|field: u32| Msg { field }"
    )]
    nested: Msg,
    #[prost(
        message,
        repeated,
        tag = "6",
        to_msgs = "|m: &Msgs| m.msgs.iter().map(|m| m.field as u32).collect::<Vec<_>>()",
        merge_msg = "|m: &mut Msgs, field: u32| m.msgs.push(Msg { field })"
    )]
    nested_repeated: Msgs,
    #[prost(
        message,
        repeated,
        tag = "7",
        to_msg = "|m: &Msg| m.field",
        from_msg = "|field: u32| Msg { field }"
    )]
    nested_repeated_vec: Vec<Msg>,
    #[prost(
        message,
        required,
        tag = "8",
        as_msg = "as_ref_unwrap",
        from_msg = "Option::Some"
    )]
    unwrap: Option<Msg>,
    #[prost(
        message,
        repeated,
        tag = "9",
        as_msgs = "iter_as_ref_unwrap",
        merge_msg = "|r: &mut Vec<Option<Msg>>, m: Msg| r.push(Some(m))",
    )]
    unwrap_repeated: Vec<Option<Msg>>,
    #[prost(
        btree_map = "uint32, uint32",
        tag = "10",
        to_msg = "(|m: &Msg| m.field)",
        from_msg = "(|field: u32| Msg { field })"
    )]
    map: BTreeMap<u32, Msg>,
    #[prost(
        message,
        repeated,
        tag = "11",
        to_msgs = "iter_msgs",
        merge_msg = "|m: &mut Vec<Msg>, field: u32| m.push(Msg { field })"
    )]
    iter: Vec<Msg>,
}

#[derive(PartialEq, prost::Message)]
struct WithoutMsgFns {
    #[prost(uint32, tag = "1")]
    tuple_left: u32,
    #[prost(string, tag = "2")]
    tuple_right: String,
    #[prost(uint32, tag = "3")]
    neg_to_pos: u32,
    #[prost(uint32, optional, tag = "4")]
    none: Option<u32>,
    #[prost(uint32, tag = "5")]
    nested: u32,
    #[prost(message, repeated, tag = "6")]
    nested_repeated: Vec<u32>,
    #[prost(message, repeated, tag = "7")]
    nested_repeated_vec: Vec<u32>,
    #[prost(message, required, tag = "8")]
    unwrap: Msg,
    #[prost(message, repeated, tag = "9")]
    unwrap_repated: Vec<Msg>,
    #[prost(btree_map = "uint32, uint32", tag = "10")]
    map: BTreeMap<u32, u32>,
    #[prost(message, repeated, tag = "11")]
    iter: Vec<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct Msg {
    #[prost(uint32, tag = "1")]
    field: u32,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Msgs {
    msgs: Vec<Msg>,
}

fn get_tuple_left(tuple: &(u32, String)) -> &u32 {
    &tuple.0
}

fn get_tuple_right(tuple: &(u32, String)) -> &String {
    &tuple.1
}

fn get_msg_field(msg: &Msg) -> &u32 {
    &msg.field
}

fn as_ref_unwrap<T>(val: &Option<T>) -> &T {
    val.as_ref().unwrap()
}

fn iter_as_ref_unwrap<T>(msgs: &Vec<Option<T>>) -> impl Iterator<Item = &T> {
    msgs.iter().map(Option::as_ref).map(Option::unwrap)
}

fn iter_msgs(msgs: &Vec<Msg>) -> impl Iterator<Item = u32> + '_ {
    msgs.iter().map(|m| m.field)
}

#[test]
fn msg_fns() {
    let mut with_msg_fns = WithMsgFns {
        tuple: (1, "foo".to_string()),
        neg_to_pos: -2,
        none: Some(3),
        nested: Msg { field: 4 },
        nested_repeated: Msgs {
            msgs: vec! [
                Msg { field: 5 },
                Msg { field: 6 },
            ],
        },
        nested_repeated_vec: vec![
            Msg { field: 7 },
            Msg { field: 8 },
        ],
        unwrap: Some(Msg { field: 9 }),
        unwrap_repeated: vec![
            Some(Msg { field: 10 }),
            Some(Msg { field: 11 }),
        ],
        map: BTreeMap::new(),
        iter: vec! [
            Msg { field: 12 },
            Msg { field: 13 },
        ],
    };

    let mut without_msg_fns = WithoutMsgFns {
        tuple_left: with_msg_fns.tuple.0,
        tuple_right: with_msg_fns.tuple.1.clone(),
        neg_to_pos: with_msg_fns.neg_to_pos.abs() as u32,
        none: None,
        nested: with_msg_fns.nested.field,
        nested_repeated: with_msg_fns
            .nested_repeated
            .msgs
            .iter()
            .map(|msg| msg.field)
            .collect(),
        nested_repeated_vec: with_msg_fns
            .nested_repeated_vec
            .iter()
            .map(|msg| msg.field)
            .collect(),
        unwrap: with_msg_fns.unwrap.clone().unwrap(),
        unwrap_repated: with_msg_fns
            .unwrap_repeated
            .iter()
            .map(|msg| msg.clone().unwrap())
            .collect(),
        map: BTreeMap::new(),
        iter: with_msg_fns
            .iter
            .iter()
            .map(|msg| msg.field)
            .collect(),
    };

    for i in 0..3 {
        let k = 14 + i * 2;
        let v = k + 1;

        with_msg_fns.map.insert(k, Msg { field: v });
        without_msg_fns.map.insert(k, v);
    }

    let mut with_msg_fns_buf = Vec::with_capacity(with_msg_fns.encoded_len());
    let mut without_msg_fns_buf = Vec::with_capacity(without_msg_fns.encoded_len());

    with_msg_fns.encode(&mut with_msg_fns_buf).expect("failed encoding");
    without_msg_fns.encode(&mut without_msg_fns_buf).expect("failed encoding");

    assert_eq!(with_msg_fns_buf, without_msg_fns_buf);

    with_msg_fns.none = None;

    assert_eq!(WithMsgFns::decode(with_msg_fns_buf.as_ref()).unwrap(), with_msg_fns);
    assert_eq!(WithMsgFns::decode(without_msg_fns_buf.as_ref()).unwrap(), with_msg_fns);

    assert_eq!(WithoutMsgFns::decode(without_msg_fns_buf.as_ref()).unwrap(), without_msg_fns);
    assert_eq!(WithoutMsgFns::decode(with_msg_fns_buf.as_ref()).unwrap(), without_msg_fns);
}
