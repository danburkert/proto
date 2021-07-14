//! Utilities for working with Protobuf paths.

use std::collections::HashMap;
use std::iter;

/// Maps a fully-qualified Protobuf path to a value using path matchers.
#[derive(Debug, Default)]
pub(crate) struct PathMap<T, A> {
    matchers: HashMap<Matcher<A>, T>,
}

/// Matches against a given path and attributes.
/// For details about matching paths see [btree_map](crate::Config#method.btree_map).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Matcher<A> {
    /// The proto path to match against.
    pub path: String,
    /// The attributes to match against.
    pub attributes: A,
}

impl<A> Matcher<A> {
    pub fn new(path: impl Into<String>, attributes: A) -> Self {
        Self {
            path: path.into(),
            attributes,
        }
    }
}

impl<I, A> From<I> for Matcher<A>
where
    I: Into<String>,
    A: Default,
{
    fn from(path: I) -> Self {
        Self {
            path: path.into(),
            attributes: A::default(),
        }
    }
}

pub(crate) trait MatcherAttributes: std::hash::Hash + Eq {
    fn matches(&self, input: &Self) -> bool;
}

impl MatcherAttributes for () {
    fn matches(&self, input: &Self) -> bool {
        true
    }
}

impl<T, A> PathMap<T, A>
where
    A: MatcherAttributes,
{
    /// Inserts a new matcher and associated value to the path map.
    pub(crate) fn insert(&mut self, matcher: Matcher<A>, value: T) {
        self.matchers.insert(matcher, value);
    }

    pub(crate) fn matches(&self, fq_path: &'_ str, field: Option<&'_ str>, attrs: &A) -> bool {
        self.matches_with_attributes(fq_path, field, None)
    }

    pub(crate) fn matches_with_attributes(
        &self,
        fq_path: &'_ str,
        field: Option<&'_ str>,
        attrs: Option<&A>,
    ) -> bool {
        self.get_with_attributes(fq_path, attrs).is_some()
            || field
                .and_then(|field_name| self.get_field_with_attributes(fq_path, field_name, attrs))
                .is_some()
    }

    /// Returns the value which matches the provided fully-qualified Protobuf path.
    pub(crate) fn get<'a>(&'a self, fq_path: &'a str) -> Option<&T> {
        self.get_with_attributes(fq_path, None)
    }

    /// Returns the value which matches the provided fully-qualified Protobuf path.
    pub(crate) fn get_with_attributes<'a>(
        &'a self,
        fq_path: &'a str,
        attrs: Option<&A>,
    ) -> Option<&T> {
        // First, try matching the full path.
        iter::once(fq_path)
            // Then, try matching path suffixes.
            .chain(suffixes(fq_path))
            // Then, try matching path prefixes.
            .chain(prefixes(fq_path))
            // Then, match the global path. This matcher must never fail, since the constructor
            // initializes it.
            .chain(iter::once("."))
            .flat_map(|path| self.matchers(path, attrs))
            .next()
    }

    fn matchers(&self, path: &'_ str, attrs: Option<&A>) -> Vec<&'_ T> {
        self.matchers
            .iter()
            .filter(|(matcher, _)| &matcher.path == path)
            .filter(|(matcher, _)| {
                if let Some(attrs) = attrs {
                    matcher.attributes.matches(attrs)
                } else {
                    true
                }
            })
            .map(|(_, t)| t)
            .collect()
    }

    /// Returns the value which matches the provided fully-qualified Protobuf path and field name.
    pub(crate) fn get_field(&self, fq_path: &'_ str, field: &'_ str) -> Option<&T> {
        self.get_field_with_attributes(fq_path, field, None)
    }

    /// Returns the value which matches the provided fully-qualified Protobuf path and field name.
    pub(crate) fn get_field_with_attributes(
        &self,
        fq_path: &'_ str,
        field: &'_ str,
        attrs: Option<&A>,
    ) -> Option<&T> {
        let full_path = format!("{}.{}", fq_path, field);
        let full_path = full_path.as_str();

        // First, try matching the path.
        let value = iter::once(full_path)
            // Then, try matching path suffixes.
            .chain(suffixes(full_path))
            // Then, try matching path suffixes without the field name.
            .chain(suffixes(fq_path))
            // Then, try matching path prefixes.
            .chain(prefixes(full_path))
            // Then, match the global path. This matcher must never fail, since the constructor
            // initializes it.
            .chain(iter::once("."))
            .flat_map(|path| self.matchers(path, attrs))
            .next();

        value
    }

    /// Removes all matchers from the path map.
    pub(crate) fn clear(&mut self) {
        self.matchers.clear();
    }
}

/// Given a fully-qualified path, returns a sequence of fully-qualified paths which match a prefix
/// of the input path, in decreasing path-length order.
///
/// Example: prefixes(".a.b.c.d") -> [".a.b.c", ".a.b", ".a"]
fn prefixes(fq_path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(fq_path), |path| {
        path.rsplitn(2, '.').nth(1).filter(|path| !path.is_empty())
    })
    .skip(1)
}

/// Given a fully-qualified path, returns a sequence of paths which match the suffix of the input
/// path, in decreasing path-length order.
///
/// Example: suffixes(".a.b.c.d") -> ["a.b.c.d", "b.c.d", "c.d", "d"]
fn suffixes(fq_path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(fq_path), |path| {
        path.splitn(2, '.').nth(1).filter(|path| !path.is_empty())
    })
    .skip(1)
}

#[cfg(test)]
mod tests {

    use crate::code_generator::FieldAttributes;

    use super::*;

    #[test]
    fn test_prefixes() {
        assert_eq!(
            prefixes(".a.b.c.d").collect::<Vec<_>>(),
            vec![".a.b.c", ".a.b", ".a"],
        );
        assert_eq!(prefixes(".a").count(), 0);
        assert_eq!(prefixes(".").count(), 0);
    }

    #[test]
    fn test_suffixes() {
        assert_eq!(
            suffixes(".a.b.c.d").collect::<Vec<_>>(),
            vec!["a.b.c.d", "b.c.d", "c.d", "d"],
        );
        assert_eq!(suffixes(".a").collect::<Vec<_>>(), vec!["a"]);
        assert_eq!(suffixes(".").collect::<Vec<_>>(), Vec::<&str>::new());
    }

    #[test]
    fn test_path_map_get() {
        let mut path_map = PathMap::<_, ()>::default();
        path_map.insert(".a.b.c.d".into(), 1);
        path_map.insert(".a.b".into(), 2);
        path_map.insert("M1".into(), 3);
        path_map.insert("M1.M2".into(), 4);
        path_map.insert("M1.M2.f1".into(), 5);
        path_map.insert("M1.M2.f2".into(), 6);

        assert_eq!(None, path_map.get(".a.other"));
        assert_eq!(None, path_map.get(".a.bother"));
        assert_eq!(None, path_map.get(".other"));
        assert_eq!(None, path_map.get(".M1.other"));
        assert_eq!(None, path_map.get(".M1.M2.other"));

        assert_eq!(Some(&1), path_map.get(".a.b.c.d"));
        assert_eq!(Some(&1), path_map.get(".a.b.c.d.other"));

        assert_eq!(Some(&2), path_map.get(".a.b"));
        assert_eq!(Some(&2), path_map.get(".a.b.c"));
        assert_eq!(Some(&2), path_map.get(".a.b.other"));
        assert_eq!(Some(&2), path_map.get(".a.b.other.Other"));
        assert_eq!(Some(&2), path_map.get(".a.b.c.dother"));

        assert_eq!(Some(&3), path_map.get(".M1"));
        assert_eq!(Some(&3), path_map.get(".a.b.c.d.M1"));
        assert_eq!(Some(&3), path_map.get(".a.b.M1"));

        assert_eq!(Some(&4), path_map.get(".M1.M2"));
        assert_eq!(Some(&4), path_map.get(".a.b.c.d.M1.M2"));
        assert_eq!(Some(&4), path_map.get(".a.b.M1.M2"));

        assert_eq!(Some(&5), path_map.get(".M1.M2.f1"));
        assert_eq!(Some(&5), path_map.get(".a.M1.M2.f1"));
        assert_eq!(Some(&5), path_map.get(".a.b.M1.M2.f1"));

        assert_eq!(Some(&6), path_map.get(".M1.M2.f2"));
        assert_eq!(Some(&6), path_map.get(".a.M1.M2.f2"));
        assert_eq!(Some(&6), path_map.get(".a.b.M1.M2.f2"));

        // get_field

        assert_eq!(Some(&2), path_map.get_field(".a.b.Other", "other"));

        assert_eq!(Some(&4), path_map.get_field(".M1.M2", "other"));
        assert_eq!(Some(&4), path_map.get_field(".a.M1.M2", "other"));
        assert_eq!(Some(&4), path_map.get_field(".a.b.M1.M2", "other"));

        assert_eq!(Some(&5), path_map.get_field(".M1.M2", "f1"));
        assert_eq!(Some(&5), path_map.get_field(".a.M1.M2", "f1"));
        assert_eq!(Some(&5), path_map.get_field(".a.b.M1.M2", "f1"));

        assert_eq!(Some(&6), path_map.get_field(".M1.M2", "f2"));
        assert_eq!(Some(&6), path_map.get_field(".a.M1.M2", "f2"));
        assert_eq!(Some(&6), path_map.get_field(".a.b.M1.M2", "f2"));
    }

    #[test]
    fn test_path_map_get_with_attributes() {
        let mut path_map = PathMap::<_, FieldAttributes>::default();
        path_map.insert(
            Matcher::new(".a", FieldAttributes::new(Some(true), Some(false))),
            1,
        );
        path_map.insert(
            Matcher::new(".b", FieldAttributes::new(Some(false), Some(false))),
            2,
        );
        path_map.insert(Matcher::new(".c", FieldAttributes::new(None, None)), 3);
        path_map.insert(
            Matcher::new(".d", FieldAttributes::new(None, Some(true))),
            4,
        );

        assert_eq!(
            Some(&1),
            path_map
                .get_with_attributes(".a", Some(&FieldAttributes::new(Some(true), Some(false))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".a", Some(&FieldAttributes::new(None, None)))
        );

        assert_eq!(
            Some(&2),
            path_map
                .get_with_attributes(".b", Some(&FieldAttributes::new(Some(false), Some(false))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".b", Some(&FieldAttributes::new(Some(false), None)))
        );
        assert_eq!(
            None,
            path_map
                .get_with_attributes(".b", Some(&FieldAttributes::new(Some(true), Some(false))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".b", Some(&FieldAttributes::new(Some(true), None)))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".b", Some(&FieldAttributes::new(None, Some(false))))
        );
        assert_eq!(
            None,
            path_map
                .get_with_attributes(".b", Some(&FieldAttributes::new(Some(false), Some(true))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".b", Some(&FieldAttributes::new(None, Some(true))))
        );

        assert_eq!(
            Some(&3),
            path_map
                .get_with_attributes(".c", Some(&FieldAttributes::new(Some(false), Some(false))))
        );
        assert_eq!(
            Some(&3),
            path_map.get_with_attributes(".c", Some(&FieldAttributes::new(Some(false), None)))
        );
        assert_eq!(
            Some(&3),
            path_map
                .get_with_attributes(".c", Some(&FieldAttributes::new(Some(true), Some(false))))
        );
        assert_eq!(
            Some(&3),
            path_map.get_with_attributes(".c", Some(&FieldAttributes::new(Some(true), None)))
        );
        assert_eq!(
            Some(&3),
            path_map.get_with_attributes(".c", Some(&FieldAttributes::new(None, Some(false))))
        );
        assert_eq!(
            Some(&3),
            path_map
                .get_with_attributes(".c", Some(&FieldAttributes::new(Some(false), Some(true))))
        );
        assert_eq!(
            Some(&3),
            path_map.get_with_attributes(".c", Some(&FieldAttributes::new(None, Some(true))))
        );

        assert_eq!(
            Some(&4),
            path_map
                .get_with_attributes(".d", Some(&FieldAttributes::new(Some(false), Some(true))))
        );
        assert_eq!(
            Some(&4),
            path_map.get_with_attributes(".d", Some(&FieldAttributes::new(Some(true), Some(true))))
        );
        assert_eq!(
            Some(&4),
            path_map.get_with_attributes(".d", Some(&FieldAttributes::new(None, Some(true))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".d", Some(&FieldAttributes::new(Some(false), None)))
        );
        assert_eq!(
            None,
            path_map
                .get_with_attributes(".d", Some(&FieldAttributes::new(Some(true), Some(false))))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".d", Some(&FieldAttributes::new(Some(true), None)))
        );
        assert_eq!(
            None,
            path_map.get_with_attributes(".d", Some(&FieldAttributes::new(None, Some(false))))
        );
    }
}
