use std::{
    collections::{btree_map, BTreeMap},
    iter::{FromIterator, Peekable},
    marker::PhantomData,
    ops::Range,
};

#[derive(Clone, Debug)]
pub struct RangeMap<Offset: Copy + Ord, Id: Copy + Ord>(BTreeMap<(Offset, Id), bool>);

impl<Offset: Copy + Ord, Id: Copy + Ord> Default for RangeMap<Offset, Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Offset: Copy + Ord, Id: Copy + Ord> RangeMap<Offset, Id> {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn insert(&mut self, range: Range<Offset>, id: Id) {
        // empty range don't need entries
        if range.start >= range.end {
            return;
        }

        self.insert_point(range.start, id, true);
        self.insert_point(range.end, id, false);
    }

    fn insert_point(&mut self, offset: Offset, id: Id, value: bool) {
        match self.0.entry((offset, id)) {
            btree_map::Entry::Occupied(entry) => {
                if *entry.get() != value {
                    entry.remove();
                }
            }
            btree_map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }

    pub fn iter<T: FromIterator<Id>>(&self) -> Iter<Offset, Id, T> {
        Iter {
            iter: self.0.iter().peekable(),
            stack: Stack::new(),
            t: PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, Offset: Copy + Ord, Id: Copy + Ord, T> {
    iter: Peekable<btree_map::Iter<'a, (Offset, Id), bool>>,
    stack: Stack<Id>,
    t: PhantomData<T>,
}

impl<'a, Offset, Id, T> Iterator for Iter<'a, Offset, Id, T>
where
    Offset: Copy + Ord,
    Id: Copy + Ord,
    T: FromIterator<Id>,
{
    type Item = (Range<Offset>, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if cfg!(debug_assertions) {
                // ensure we end up with an empty stack
                if self.iter.peek().is_none() {
                    assert!(self.stack.is_empty(), "invalid stack state");
                }
            }

            let stack = &mut self.stack;

            let (key, enabled) = self.iter.next()?;
            let (offset, id) = *key;
            stack.insert(id, *enabled);

            let mut end = offset;

            while self
                .iter
                .next_if(|(key, enabled)| {
                    let (next_offset, id) = **key;
                    let enabled = **enabled;

                    // find the exclusive end of the iteration
                    end = next_offset;

                    // collect all of the entries at the current offset
                    if next_offset == offset {
                        stack.insert(id, enabled);
                        return true;
                    }

                    // only keep going if the stack doesn't change sizes
                    stack.maybe_insert(id, enabled)
                })
                .is_some()
            {}

            // don't return empty lists
            if self.stack.is_empty() {
                continue;
            }

            let range = offset..end;
            let value = self.stack.ids().collect();

            return Some((range, value));
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Stack<Id: Copy + Ord>(BTreeMap<Id, u32>);

impl<Id: Copy + Ord> Stack<Id> {
    fn new() -> Self {
        Stack(BTreeMap::new())
    }

    fn insert(&mut self, id: Id, enabled: bool) -> bool {
        if enabled {
            match self.0.entry(id) {
                btree_map::Entry::Occupied(mut entry) => {
                    let value = entry.get_mut();
                    *value += 1;
                    false
                }
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(1);
                    true
                }
            }
        } else {
            match self.0.entry(id) {
                btree_map::Entry::Occupied(mut entry) => {
                    let value = entry.get_mut();
                    *value -= 1;

                    if *value == 0 {
                        entry.remove();
                        return true;
                    }

                    false
                }
                btree_map::Entry::Vacant(_) => {
                    debug_assert!(false, "invalid stack state");
                    false
                }
            }
        }
    }

    fn maybe_insert(&mut self, id: Id, enabled: bool) -> bool {
        if enabled {
            match self.0.entry(id) {
                btree_map::Entry::Occupied(mut entry) => {
                    let value = entry.get_mut();
                    *value += 1;
                    true
                }
                btree_map::Entry::Vacant(_) => {
                    // This would add a new entry
                    false
                }
            }
        } else {
            match self.0.entry(id) {
                btree_map::Entry::Occupied(mut entry) => {
                    let value = entry.get_mut();

                    if *value == 1 {
                        // this would remove an entry
                        false
                    } else {
                        *value -= 1;
                        true
                    }
                }
                btree_map::Entry::Vacant(_) => {
                    debug_assert!(false, "invalid stack state");
                    true
                }
            }
        }
    }

    fn ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.0.keys().copied()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map<const L: usize>(ranges: [(Range<u32>, u8); L]) -> Vec<(Range<u32>, Vec<u8>)> {
        let mut map = RangeMap::default();

        for (range, id) in ranges.iter() {
            map.insert(range.start..range.end, *id);
        }

        map.iter().collect()
    }

    macro_rules! snapshot_test {
        ($name:ident, [$(($range:expr, $id:expr)),* $(,)?]) => {
            #[test]
            fn $name() {
                insta::assert_debug_snapshot!(stringify!($name), map([$(($range, $id)),*]));
            }
        };
    }

    snapshot_test!(empty, []);
    snapshot_test!(single, [(0..4, 0)]);
    snapshot_test!(nested, [(0..10, 0), (4..5, 0)]);
    snapshot_test!(contiguous, [(0..4, 0), (4..10, 0)]);
    snapshot_test!(contiguous_reverse, [(4..10, 0), (0..4, 0)]);
    snapshot_test!(non_contiguous, [(0..4, 0), (6..10, 0)]);
    snapshot_test!(overlap, [(0..6, 0), (4..10, 0)]);
    snapshot_test!(multiple, [(0..10, 0), (4..5, 1)]);
    snapshot_test!(multiple_reverse, [(4..5, 1), (0..10, 0)]);
}
