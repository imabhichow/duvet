use core::{
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};
use dashmap::DashMap;
use std::{borrow::Borrow, hash::Hash, ops::Deref, path::Path, sync::Arc};

#[derive(Debug)]
pub struct Intern<V>
where
    V: Hash + Eq,
{
    value_to_id: DashMap<Value<V>, Id>,
    id_to_value: DashMap<Id, Value<V>>,
    id: AtomicU32,
}

impl<V> Default for Intern<V>
where
    V: Hash + Eq,
{
    fn default() -> Self {
        Self {
            value_to_id: DashMap::new(),
            id_to_value: DashMap::new(),
            id: AtomicU32::new(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(u32);

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Value<V>(Arc<V>);

impl<V> Clone for Value<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> AsRef<V> for Value<V> {
    fn as_ref(&self) -> &V {
        &self.0
    }
}

impl<V> Deref for Value<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Ref<'a, V>(dashmap::mapref::one::Ref<'a, Id, Value<V>>);

impl<'a, V> AsRef<V> for Ref<'a, V> {
    fn as_ref(&self) -> &V {
        self.0.value()
    }
}

impl<'a, V> Deref for Ref<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.0.value()
    }
}

impl<V: Borrow<str>> Borrow<str> for Value<V> {
    fn borrow(&self) -> &str {
        self.0.as_ref().borrow()
    }
}

impl<V: Borrow<[u8]>> Borrow<[u8]> for Value<V> {
    fn borrow(&self) -> &[u8] {
        self.0.as_ref().borrow()
    }
}

impl<V: Borrow<Path>> Borrow<Path> for Value<V> {
    fn borrow(&self) -> &Path {
        self.0.as_ref().borrow()
    }
}

impl<'a, V: fmt::Debug> fmt::Debug for Ref<'a, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<V> Intern<V>
where
    V: Hash + Eq,
{
    pub fn intern<Q: ?Sized>(&self, key: &Q) -> Id
    where
        Q: Hash + Eq + ToOwned<Owned = V>,
        Value<V>: Borrow<Q>,
    {
        if let Some(id) = self.value_to_id.get(key) {
            *id
        } else {
            let id = Id(self.id.fetch_add(1, Ordering::SeqCst));
            let value = Value(Arc::new(key.to_owned()));
            self.value_to_id.insert(value.clone(), id);
            self.id_to_value.insert(id, value);
            id
        }
    }

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        Q: Hash + Eq + ToOwned<Owned = V>,
        Value<V>: Borrow<Q>,
    {
        self.value_to_id.contains_key(key)
    }

    pub fn resolve(&self, id: Id) -> Ref<V> {
        Ref(self.id_to_value.get(&id).expect("invalid id"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_test() {
        let intern = Intern::default();

        let id1 = intern.intern("value");
        let id2 = intern.intern("other");
        let id3 = intern.intern("value");
        assert_eq!(id1, id3);
        assert_ne!(id1, id2);

        let value = intern.resolve(id1);
        assert_eq!(value.as_ref(), "value");
    }
}
