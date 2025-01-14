use itertools::Itertools;

// Invariant: should always begin with a '.' (dot)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct FullyQualifiedName(String);

impl FullyQualifiedName {
    pub fn new(package_string: &str, type_path: &[impl AsRef<str>], message_name: &str) -> Self {
        Self(format!(
            "{}{}{}{}{}{}",
            if package_string.is_empty() { "" } else { "." },
            package_string.trim_matches('.'),
            if type_path.is_empty() { "" } else { "." },
            type_path
                .iter()
                .map(AsRef::as_ref)
                .map(|type_path_str| type_path_str.trim_start_matches('.'))
                .join("."),
            if message_name.is_empty() { "" } else { "." },
            message_name,
        ))
    }

    pub fn from_type_name(type_name: &str) -> Self {
        Self::new("", &[type_name], "")
    }

    pub fn path_iterator(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0[1..].split('.')
    }

    pub fn join(&self, path: &str) -> Self {
        Self(format!("{}.{}", self.0, path))
    }
}

impl AsRef<str> for FullyQualifiedName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    impl From<&str> for FullyQualifiedName {
        fn from(str: &str) -> Self {
            Self(str.to_string())
        }
    }
}
