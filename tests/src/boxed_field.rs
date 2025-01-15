include!(concat!(env!("OUT_DIR"), "/boxed_field.rs"));

cfg_if! {
    if #[cfg(feature = "edition-2015")] {
        use boxed_field::foo::OneofField;
    } else {
        use foo::OneofField;
    }
}

#[test]
/// Confirm `Foo::bar` and `OneofField::BoxQux` is boxed by creating an instance
fn test_boxed_field() {
    use alloc::boxed::Box;
    let _ = Foo {
        bar: Some(Box::new(Bar {})),
        oneof_field: Some(OneofField::BoxQux(Box::new(Bar {}))),
    };
}
