use std::path::PathBuf;

fn main() {
    env_logger::init();

    let src = PathBuf::from("src");
    let includes = &[src.clone()];

    let mut config = prost_build::Config::new();
    config
        .btree_map(["."])
        .compile_protos(&[src.join("encoded_len.proto")], includes)
        .unwrap();
}
