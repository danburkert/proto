// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Container {
    #[prost(oneof="container::Data", tags="1, 2")]
    pub data: ::core::option::Option<container::Data>,
}
/// Nested message and enum types in `Container`.
pub mod container {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag="1")]
        Foo(::prost::alloc::boxed::Box<super::Foo>),
        #[prost(message, tag="2")]
        Bar(super::Bar),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Foo {
    #[prost(string, tag="1")]
    pub foo: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Bar {
    #[prost(message, optional, boxed, tag="1")]
    pub qux: ::core::option::Option<::prost::alloc::boxed::Box<Qux>>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Qux {
}
