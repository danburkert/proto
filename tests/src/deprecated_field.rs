#[cfg(not(feature = "std"))]
use alloc::string::ToString;

mod deprecated_field {
    // #![deny(unused_results)]
    include!(concat!(env!("OUT_DIR"), "/deprecated_field.rs"));
}

#[test]
fn test_warns_when_using_fields_with_deprecated_field() {
    #[allow(deprecated)]
    let message = deprecated_field::Test {
        not_outdated: ".ogg".to_string(),
        outdated: ".wav".to_string(),
        test3: Some(deprecated_field::test::Test3::OneofOutdated(
            ".mp3".to_string(),
        )),
    };
    #[allow(deprecated)]
    let enum_ = deprecated_field::Test2::Outdated;
    // This test relies on the `#[allow(deprecated)]` attribute to ignore the warning that should
    // be raised by the compiler.
    // This test has a shortcoming since it doesn't explicitly check for the presence of the
    // `deprecated` attribute since it doesn't exist at runtime. If complied without the `allow`
    // attribute the following warning would be raised:
    //
    //    warning: use of deprecated item 'deprecated_field::deprecated_field::Test::outdated'
    //      --> tests/src/deprecated_field.rs:11:9
    //       |
    //    11 |         outdated: ".wav".to_string(),
    //       |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //       |
    //       = note: `#[warn(deprecated)]` on by default
    drop(message);
    // Call drop on type implement copy will raise warning, disable it for not relative
    #[allow(dropping_copy_types)]
    drop(enum_);
}
