mod proto {
    include!(concat!(env!("OUT_DIR"), "/encoded_len.rs"));
}

mod limit;
mod overflow;
