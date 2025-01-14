use prost::Name;

include!(concat!(env!("OUT_DIR"), "/type_names.rs"));

#[test]
fn valid_type_names() {
    assert_eq!("Foo", Foo::NAME);
    assert_eq!("type_names", Foo::PACKAGE);
    assert_eq!("type_names.Foo", Foo::full_name());
    assert_eq!("tests/type_names.Foo", Foo::type_url());

    assert_eq!("Bar", foo::Bar::NAME);
    assert_eq!("type_names", foo::Bar::PACKAGE);
    assert_eq!("type_names.Foo.Bar", foo::Bar::full_name());
    assert_eq!("tests/type_names.Foo.Bar", foo::Bar::type_url());

    assert_eq!("Baz", Baz::NAME);
    assert_eq!("type_names", Baz::PACKAGE);
    assert_eq!("type_names.Baz", Baz::full_name());
    assert_eq!("/type_names.Baz", Baz::type_url());

    assert_eq!("Qux", Qux::NAME);
    assert_eq!("type_names", Qux::PACKAGE);
    assert_eq!("type_names.Qux", Qux::full_name());
    assert_eq!("tests-cumulative/type_names.Qux", Qux::type_url());
}
