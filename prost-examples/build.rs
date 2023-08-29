fn main() {
    prost_build::Config::new()
        .generate_unknown_fields()
        .compile_protos(&["hello.proto"], &["."])
        .unwrap();
}
