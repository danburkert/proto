// protoc on Windows outputs \r\n line endings.
#![cfg(not(windows))]
#![cfg(feature = "std")]

use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::string::String;

/// Test which bootstraps protobuf.rs and compiler.rs from the .proto definitions in the Protobuf
/// repo. Ensures that the checked-in compiled versions are up-to-date.
#[test]
fn bootstrap() {
    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("src")
        .join("include");
    let protobuf = include.join("google").join("protobuf");

    let tempdir = tempfile::Builder::new()
        .prefix("prost-types-bootstrap")
        .tempdir()
        .unwrap();

    prost_build::Config::new()
        .compile_well_known_types()
        .btree_map(["."])
        .out_dir(tempdir.path())
        .compile_protos(
            &[
                // Protobuf Plugins.
                protobuf.join("descriptor.proto"),
                protobuf.join("compiler").join("plugin.proto"),
                // Well-known Types (except wrapper.proto, whose types are substituted
                // for the corresponding standard library types).
                protobuf.join("any.proto"),
                protobuf.join("api.proto"),
                protobuf.join("duration.proto"),
                protobuf.join("field_mask.proto"),
                protobuf.join("source_context.proto"),
                protobuf.join("struct.proto"),
                protobuf.join("timestamp.proto"),
                protobuf.join("type.proto"),
            ],
            &[include],
        )
        .unwrap();

    let mut bootstrapped_protobuf = String::new();
    fs::File::open(tempdir.path().join("google.protobuf.rs"))
        .unwrap()
        .read_to_string(&mut bootstrapped_protobuf)
        .unwrap();

    let mut bootstrapped_compiler = String::new();
    fs::File::open(tempdir.path().join("google.protobuf.compiler.rs"))
        .unwrap()
        .read_to_string(&mut bootstrapped_compiler)
        .unwrap();

    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("no parent")
        .join("prost-types")
        .join("src");

    let mut protobuf = String::new();
    fs::File::open(src.join("protobuf.rs"))
        .unwrap()
        .read_to_string(&mut protobuf)
        .unwrap();

    let mut compiler = String::new();
    fs::File::open(src.join("compiler.rs"))
        .unwrap()
        .read_to_string(&mut compiler)
        .unwrap();

    if protobuf != bootstrapped_protobuf {
        fs::File::create(src.join("protobuf.rs"))
            .unwrap()
            .write_all(bootstrapped_protobuf.as_bytes())
            .unwrap();
    }
    if compiler != bootstrapped_compiler {
        fs::File::create(src.join("compiler.rs"))
            .unwrap()
            .write_all(bootstrapped_compiler.as_bytes())
            .unwrap();
    }

    assert_eq!(protobuf, bootstrapped_protobuf);
    assert_eq!(compiler, bootstrapped_compiler);
}
