//!#[doc(hidden)]

/// # Doc tests
///
/// A message is annotated in the build settings with `#[non_exhaustive]`
/// to prevent external crate users from writing code that will break when
/// more fields are eventually added to the message definition.
///
/// So, the struct expression syntax cannot be used:
///
/// ```compile_fail
/// # use tests::builders::builders::EmptyForNow;
/// let message = EmptyForNow {};
/// ```
///
/// This forward-compatible builder code can be used instead:
///
/// ```
/// # use tests::builders::builders::EmptyForNow;
/// let message = EmptyForNow::builder().build();
/// ```
///
/// However, the builder is not the most concise way to create a message
/// with all default field values, as long as `prost-build` has `Default`
/// derived for the generated message structs:
///
/// ```
/// # use tests::builders::builders::EmptyForNow;
/// let message = EmptyForNow::default();
/// ```
///
/// In the (hopefully rare) case when a message has a field named `builder`
/// with the type and labels that cause the `Message` derive generate a getter
/// method for it, the `builder` associated function cannot be generated
/// because it will come into conflict with the getter method, so our builder
/// configuration provides a different name:
///
/// ```
/// # use tests::builders::builders::ConflictProneScalar;
/// let msg = ConflictProneScalar::fields().build();
/// assert_eq!(msg.builder(), 0);
/// ```
///
/// ```
/// # use tests::builders::builders::{ConflictProneEnum, AnEnum};
/// let msg = ConflictProneEnum::fields().builder(AnEnum::A).build();
/// assert_eq!(msg.builder(), AnEnum::A);
/// ```
///
/// The `Default` implementation for the builder type can also be used:
///
/// ```
/// # use tests::builders::builders::conflict_prone_scalar;
/// let msg = conflict_prone_scalar::Fields::default().builder(42).build();
/// assert_eq!(msg.builder(), 42);
/// ```
///
/// A plain non-optional scalar field named `builder` should not be problematic,
/// as it has no getter method generated (even though the resulting builder API
/// is confusing):
///
/// ```
/// # use tests::builders::builders::ConflictFreeScalar;
/// let msg = ConflictFreeScalar::builder().builder("hello").build();
/// assert_eq!(msg.builder, "hello");
/// ```
///
/// A Rust keyword used as a field name causes the generated setter name to
/// be prefixed with r#:
///
/// ```
/// # use tests::builders::builders::Keywordy;
/// let msg = Keywordy::builder().r#type("foo").build();
/// assert_eq!(msg.r#type, "foo");
/// ```
///
pub mod builders {
    include!(concat!(env!("OUT_DIR"), "/builders.rs"));
}

#[cfg(test)]
mod tests {
    use super::builders::{zoo, AnEnum, EmptyForNow, Evolved, Zoo};

    use alloc::boxed::Box;
    use alloc::vec::Vec;

    #[test]
    fn added_field_does_not_break_builder_init() {
        let v = Evolved::builder().initial_field(42).build();
        assert_eq!(v.initial_field, 42);
        assert!(v.added_field.is_empty());
    }

    #[test]
    fn repeated_field_init_from_iterator() {
        let v = Evolved::builder().added_field(["hello", "world"]).build();
        assert_eq!(v.initial_field, 0);
        assert_eq!(v.added_field.len(), 2);
    }

    #[test]
    fn message_field_init() {
        let msg = Zoo::builder()
            .message_field(Evolved::builder().initial_field(42))
            .build();
        assert_eq!(
            msg.message_field,
            Some(Evolved {
                initial_field: 42,
                added_field: Vec::new(),
            })
        );
    }

    #[test]
    fn enum_field_setter_accepts_enum_values() {
        let msg = Zoo::builder().enum_field(AnEnum::B).build();
        assert_eq!(msg.enum_field, AnEnum::B as i32);
    }

    #[test]
    fn enum_field_default() {
        let msg = Zoo::builder().build();
        assert_eq!(msg.enum_field, 0);
    }

    #[test]
    fn boxed_field_setter_accepts_unboxed_values() {
        let msg = Zoo::builder().boxed_field(EmptyForNow::default()).build();
        assert_eq!(msg.boxed_field, Some(Box::default()));
    }

    #[test]
    fn oneof_field_setter_takes_enum_values() {
        let msg = Zoo::builder().oneof_field(zoo::OneofField::A(42)).build();
        assert_eq!(msg.oneof_field, Some(zoo::OneofField::A(42)));
    }
}
