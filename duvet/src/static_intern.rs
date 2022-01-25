#[macro_export]
macro_rules! static_intern {
    ($name:ident, $id:ident) => {
        static_intern!($name, $id, u32);
    };
    ($name:ident, $id:ident, $id_ty:ty) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name {
            strings: std::sync::Arc<[arcstr::Substr]>,
        }

        impl $name {
            pub fn resolve(&self, value: &str) -> Option<$id> {
                self.strings
                    .binary_search_by(|v| v.as_str().cmp(value))
                    .ok()
                    .map(|id| $id(id as _))
            }

            pub fn get(&self, id: $id) -> Option<&arcstr::Substr> {
                self.strings.get(id.0 as usize)
            }
        }

        impl<T> core::iter::FromIterator<T> for $name
        where
            arcstr::Substr: From<T>,
        {
            fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                let mut strings: Vec<_> = iter.into_iter().map(|s| s.into()).collect();
                strings.sort();
                strings.dedup();
                debug_assert!(<$id_ty>::MAX as usize >= strings.len());
                let strings = std::sync::Arc::from(strings);
                Self { strings }
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $id($id_ty);
    };
}

#[cfg(test)]
mod tests {
    use duvet_core::fs::arcstr::literal;

    #[test]
    fn resolve_test() {
        static_intern!(Intern, Id);

        let intern: Intern = [literal!("hello"), literal!("world!")].iter().collect();

        let hello = intern.resolve("hello").unwrap();
        let world = intern.resolve("world!").unwrap();

        assert_ne!(hello, world);
        assert!(intern.resolve("other").is_none());
    }
}
