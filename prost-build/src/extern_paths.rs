use std::collections::{hash_map, HashMap};

use itertools::Itertools;

use crate::{
    fully_qualified_name::FullyQualifiedName,
    ident::{to_snake, to_upper_camel},
};

fn validate_proto_path(path: &str) -> Result<(), String> {
    if path.chars().next().map(|c| c != '.').unwrap_or(true) {
        return Err(format!(
            "Protobuf paths must be fully qualified (begin with a leading '.'): {}",
            path
        ));
    }
    if path.split('.').skip(1).any(str::is_empty) {
        return Err(format!("invalid fully-qualified Protobuf path: {}", path));
    }
    Ok(())
}

#[derive(Debug)]
pub struct ExternPaths {
    // IMPROVEMENT: store as FullyQualifiedName and syn::Path
    extern_paths: HashMap<String, String>,
}

impl ExternPaths {
    pub fn new<'a>(
        paths: impl IntoIterator<Item = (&'a str, &'a str)> + 'a,
        prost_types: bool,
    ) -> Result<ExternPaths, String> {
        let mut extern_paths = ExternPaths {
            extern_paths: HashMap::new(),
        };

        for (proto_path, rust_path) in paths {
            extern_paths.insert(proto_path, rust_path)?;
        }

        if prost_types {
            extern_paths.insert(".google.protobuf", "::prost_types")?;
            extern_paths.insert(".google.protobuf.BoolValue", "bool")?;
            extern_paths.insert(
                ".google.protobuf.BytesValue",
                "::prost::alloc::vec::Vec<u8>",
            )?;
            extern_paths.insert(".google.protobuf.DoubleValue", "f64")?;
            extern_paths.insert(".google.protobuf.Empty", "()")?;
            extern_paths.insert(".google.protobuf.FloatValue", "f32")?;
            extern_paths.insert(".google.protobuf.Int32Value", "i32")?;
            extern_paths.insert(".google.protobuf.Int64Value", "i64")?;
            extern_paths.insert(
                ".google.protobuf.StringValue",
                "::prost::alloc::string::String",
            )?;
            extern_paths.insert(".google.protobuf.UInt32Value", "u32")?;
            extern_paths.insert(".google.protobuf.UInt64Value", "u64")?;
        }

        Ok(extern_paths)
    }

    fn insert(
        &mut self,
        proto_path: impl Into<String>,
        rust_path: impl Into<String>,
    ) -> Result<(), String> {
        let proto_path = proto_path.into();
        let rust_path = rust_path.into();

        validate_proto_path(&proto_path)?;
        match self.extern_paths.entry(proto_path) {
            hash_map::Entry::Occupied(occupied) => {
                return Err(format!(
                    "duplicate extern Protobuf path: {}",
                    occupied.key()
                ));
            }
            hash_map::Entry::Vacant(vacant) => vacant.insert(rust_path),
        };
        Ok(())
    }

    pub fn resolve_ident(&self, pb_ident: &FullyQualifiedName) -> Option<String> {
        let pb_ident = pb_ident.as_ref();
        if let Some(rust_path) = self.extern_paths.get(pb_ident) {
            return Some(rust_path.clone());
        }

        // TODO(danburkert): there must be a more efficient way to do this, maybe a trie?
        for (idx, _) in pb_ident.rmatch_indices('.') {
            if let Some(rust_path) = self.extern_paths.get(&pb_ident[..idx]) {
                let mut segments = pb_ident[idx + 1..].split('.');
                let ident_type = segments.next_back().map(to_upper_camel);

                return Some(
                    rust_path
                        .split("::")
                        .chain(segments)
                        .enumerate()
                        .map(|(idx, segment)| {
                            if idx == 0 && segment == "crate" {
                                // If the first segment of the path is 'crate', then do not escape
                                // it into a raw identifier, since it's being used as the keyword.
                                segment.to_owned()
                            } else {
                                to_snake(segment)
                            }
                        })
                        .chain(ident_type.into_iter())
                        .join("::"),
                );
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_extern_paths() {
        let paths = ExternPaths::new(
            [
                (".foo", "::foo1"),
                (".foo.bar", "::foo2"),
                (".foo.baz", "::foo3"),
                (".foo.Fuzz", "::foo4::Fuzz"),
                (".a.b.c.d.e.f", "::abc::def"),
            ],
            false,
        )
        .unwrap();

        let case = |proto_ident: &str, resolved_ident: &str| {
            assert_eq!(
                paths.resolve_ident(&proto_ident.into()).unwrap(),
                resolved_ident
            );
        };

        case(".foo", "::foo1");
        case(".foo.Foo", "::foo1::Foo");
        case(".foo.bar", "::foo2");
        case(".foo.Bas", "::foo1::Bas");

        case(".foo.bar.Bar", "::foo2::Bar");
        case(".foo.Fuzz.Bar", "::foo4::fuzz::Bar");

        case(".a.b.c.d.e.f", "::abc::def");
        case(".a.b.c.d.e.f.g.FooBar.Baz", "::abc::def::g::foo_bar::Baz");

        assert!(paths.resolve_ident(&".a".into()).is_none());
        assert!(paths.resolve_ident(&".a.b".into()).is_none());
        assert!(paths.resolve_ident(&".a.c".into()).is_none());
    }

    #[test]
    fn test_well_known_types() {
        let paths = ExternPaths::new([], true).unwrap();

        let case = |proto_ident: &str, resolved_ident: &str| {
            assert_eq!(
                paths.resolve_ident(&proto_ident.into()).unwrap(),
                resolved_ident
            );
        };

        case(".google.protobuf.Value", "::prost_types::Value");
        case(".google.protobuf.Duration", "::prost_types::Duration");
        case(".google.protobuf.Empty", "()");
    }

    #[test]
    fn test_error_fully_qualified() {
        let paths = [("foo", "bar")];
        let err = ExternPaths::new(paths, false).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Protobuf paths must be fully qualified (begin with a leading '.'): foo"
        )
    }

    #[test]
    fn test_error_invalid_path() {
        let paths = [(".foo.", "bar")];
        let err = ExternPaths::new(paths, false).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid fully-qualified Protobuf path: .foo."
        )
    }

    #[test]
    fn test_error_duplicate() {
        let paths = [(".foo", "bar"), (".foo", "bar")];
        let err = ExternPaths::new(paths, false).unwrap_err();
        assert_eq!(err.to_string(), "duplicate extern Protobuf path: .foo")
    }
}
