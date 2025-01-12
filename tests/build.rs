#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(feature = "edition-2015")] {
        extern crate env_logger;
        extern crate prost_build;
    }
}

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    // The source directory. The indirection is necessary in order to support the tests-2015 crate,
    // which sets the current directory to tests-2015 during build script evaluation.
    let src = PathBuf::from("../tests/src");
    let includes = &[src.clone()];

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    let mut config = prost_build::Config::new();
    config.btree_map(["."]);
    // Tests for custom attributes
    config.type_attribute("Foo.Bar_Baz.Foo_barBaz", "#[derive(Eq, PartialOrd, Ord)]");
    config.type_attribute(
        "Foo.Bar_Baz.Foo_barBaz.fuzz_buster",
        "#[derive(Eq, PartialOrd, Ord)]",
    );
    config.type_attribute(
        "Foo.Custom.OneOfAttrs.Msg.field",
        "#[derive(Eq, PartialOrd, Ord)]",
    );

    config.file_descriptor_set_path(
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"))
            .join("file_descriptor_set.bin"),
    );

    config
        .compile_protos(&[src.join("ident_conversion.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .btree_map(["."])
        .compile_protos(&[src.join("nesting.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("recursive_oneof.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .type_attribute("custom_attributes.Msg", "#[allow(missing_docs)]")
        .type_attribute("custom_attributes.Msg.field", "/// Oneof docs")
        .type_attribute("custom_attributes.AnEnum", "#[allow(missing_docs)]")
        .type_attribute("custom_attributes.AnotherEnum", "/// Oneof docs")
        .field_attribute("custom_attributes.AnotherEnum.C", "/// The C docs")
        .field_attribute("custom_attributes.AnotherEnum.D", "/// The D docs")
        .field_attribute("custom_attributes.Msg.field.a", "/// Oneof A docs")
        .field_attribute("custom_attributes.Msg.field.b", "/// Oneof B docs")
        .compile_protos(&[src.join("custom_attributes.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("oneof_attributes.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("no_shadowed_types.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("no_unused_results.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .compile_protos(&[src.join("default_enum_value.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("enum_keyword_variant.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("groups.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("deprecated_field.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("derive_copy.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("default_string_escape.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .skip_debug(["custom_debug.Msg"])
        .compile_protos(&[src.join("custom_debug.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("result_enum.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("result_struct.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("option_enum.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("option_struct.proto")], includes)
        .unwrap();

    config
        .compile_protos(&[src.join("submessage_without_package.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&[src.join("proto3_presence.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .disable_comments(["."])
        .compile_protos(&[src.join("disable_comments.proto")], includes)
        .unwrap();

    config
        .bytes(["."])
        .compile_protos(&[src.join("well_known_types.proto")], includes)
        .unwrap();

    let out = std::env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out).join("wellknown_include");

    std::fs::create_dir_all(&out_path).unwrap();

    prost_build::Config::new()
        .bytes(["."])
        .out_dir(out_path)
        .include_file("wellknown_include.rs")
        .compile_protos(&[src.join("well_known_types.proto")], includes)
        .unwrap();

    config
        .compile_protos(
            &[src.join("packages/widget_factory.proto")],
            &[src.join("packages")],
        )
        .unwrap();

    prost_build::Config::new()
        .enable_type_names()
        .type_name_domain([".type_names.Foo"], "tests")
        .compile_protos(&[src.join("type_names.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .boxed("Foo.bar")
        .compile_protos(&[src.join("boxed_field.proto")], includes)
        .unwrap();

    prost_build::Config::new()
        .cowed(".cowed_field.Foo")
        .bytes([".cowed_field.Bar.myBarBytes"])
        .cowed(".cowed_field.Bar.myBarCowBytes")
        .cowed(".cowed_field.Bar.myBarCowStr")
        .cowed(".cowed_field.Bar.myNormalCowMap")
        .cowed(".cowed_field.Bar.myBtreeCowMap")
        .btree_map([
            ".cowed_field.Bar.myBtreeMap",
            ".cowed_field.Bar.myBtreeCowMap",
        ])
        .compile_protos(&[src.join("cowed_field.proto")], includes)
        .unwrap();

    // Check that attempting to compile a .proto without a package declaration does not result in an error.
    config
        .compile_protos(&[src.join("no_package.proto")], includes)
        .unwrap();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));

    // Check that attempting to compile a .proto without a package declaration succeeds.
    let no_root_packages = out_dir.as_path().join("no_root_packages");

    fs::create_dir_all(&no_root_packages).expect("failed to create prefix directory");
    let mut no_root_packages_config = prost_build::Config::new();
    no_root_packages_config
        .out_dir(&no_root_packages)
        .default_package_filename("__.default")
        .include_file("__.include.rs")
        .compile_protos(
            &[src.join("no_root_packages/widget_factory.proto")],
            &[src.join("no_root_packages")],
        )
        .unwrap();

    // Check that attempting to compile a .proto without a package declaration succeeds.
    let no_root_packages_with_default = out_dir.as_path().join("no_root_packages_with_default");

    fs::create_dir_all(&no_root_packages_with_default).expect("failed to create prefix directory");
    let mut no_root_packages_config = prost_build::Config::new();
    no_root_packages_config
        .out_dir(&no_root_packages_with_default)
        .compile_protos(
            &[src.join("no_root_packages/widget_factory.proto")],
            &[src.join("no_root_packages")],
        )
        .unwrap();

    assert!(no_root_packages_with_default.join("_.rs").exists());

    let extern_paths = out_dir.as_path().join("extern_paths");
    fs::create_dir_all(&extern_paths).expect("failed to create prefix directory");

    config.out_dir(&extern_paths);

    // Compile some of the module examples as an extern path. The extern path syntax is edition
    // specific, since the way crate-internal fully qualified paths has changed.
    cfg_if! {
        if #[cfg(feature = "edition-2015")] {
            const EXTERN_PATH: &str = "::packages::gizmo";
        } else {
            const EXTERN_PATH: &str = "crate::packages::gizmo";
        }
    };
    config.extern_path(".packages.gizmo", EXTERN_PATH);

    config
        .compile_protos(
            &[src.join("packages").join("widget_factory.proto")],
            &[src.join("packages")],
        )
        .unwrap();

    // Run the last command again, while skipping the protoc run.  Since file_descriptor_set_path
    // has been set, it will already exist, and should produce the same result.  The inputs are also
    // ignored, so provide fake input.
    config
        .skip_protoc_run()
        .compile_protos(&[] as &[&str], &[] as &[&str])
        .unwrap();
}
