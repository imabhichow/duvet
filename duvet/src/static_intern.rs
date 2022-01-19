use duvet_core::fs::ArcStr;
use std::{iter::FromIterator, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Intern {
    strings: Arc<[ArcStr]>,
}

impl Intern {
    #[inline]
    pub fn resolve(&self, value: &str) -> Option<Id> {
        self.strings
            .binary_search_by(|v| v.as_str().cmp(value))
            .ok()
            .map(|id| Id(id as _))
    }

    #[inline]
    pub fn get(&self, id: Id) -> Option<&ArcStr> {
        self.strings.get(id.0 as usize)
    }
}

impl<T> FromIterator<T> for Intern
where
    ArcStr: From<T>,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut strings: Vec<_> = iter.into_iter().map(|s| s.into()).collect();
        strings.sort();
        strings.dedup();
        let strings = Arc::from(strings);
        Intern { strings }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(u32);

#[cfg(test)]
mod tests {
    use super::*;
    use duvet_core::fs::arcstr::literal;

    #[test]
    fn resolve_test() {
        let intern: Intern = [literal!("hello"), literal!("world!")].iter().collect();

        let hello = intern.resolve("hello").unwrap();
        let world = intern.resolve("world!").unwrap();

        assert_ne!(hello, world);
        assert!(intern.resolve("other").is_none());
    }
}
