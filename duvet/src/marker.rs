use crate::schema::{EntityId, FileId, Id, IdSet, IdSetExt};
use anyhow::Result;
use byteorder::BigEndian as BE;
use core::{fmt, ops::Range};
use sled::Tree;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U32};

#[repr(C)]
#[derive(AsBytes, FromBytes, Unaligned, Clone, Copy, Debug)]
pub(crate) struct Marker {
    pub id: U32<BE>,
    pub end: U32<BE>,
}

pub(crate) struct Markers {
    markers: Tree,
}

impl fmt::Debug for Markers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Markers")
            .field("len", &self.markers.len())
            .finish()
    }
}

impl Markers {
    pub fn new(markers: Tree) -> Self {
        let m = Self { markers };
        m.init();
        m
    }

    fn init(&self) {
        self.markers.set_merge_operator(Self::merge_entry);
    }

    pub(crate) fn mark<I: Id>(&self, file: FileId, bytes: Range<u32>, id: I) -> Result<()> {
        let start = bytes.start;
        let end = bytes.end;

        if start >= end {
            return Ok(());
        }

        let marker = Marker {
            id: id.into_inner(),
            end: U32::new(end),
        };

        self.markers
            .merge((file, bytes.start).join(), marker.as_bytes())?;
        self.markers
            .merge((file, bytes.end).join(), marker.as_bytes())?;

        Ok(())
    }

    pub(crate) fn for_each<F>(&self, file: FileId, on_entry: F) -> Result<()>
    where
        F: FnMut(Entry) -> Result<()>,
    {
        let mut finisher = Finisher::new(file, on_entry);

        let mut end_offset = 0;
        for marker in self
            .markers
            .range((file, 0).join()..=(file, u32::MAX).join())
        {
            let (key, value) = marker?;
            let (_file, offset): (FileId, u32) = key.keys();
            let markers = <LayoutVerified<_, [Marker]>>::new_slice_unaligned(value.as_ref())
                .unwrap()
                .into_slice();

            finisher.on_markers(offset, markers)?;
            end_offset = offset;
        }

        finisher.flush(end_offset)?;

        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn merge_entry(_key: &[u8], old_value: Option<&[u8]>, merged_bytes: &[u8]) -> Option<Vec<u8>> {
        let mut value = old_value
            .map(Vec::from)
            .unwrap_or_else(|| Vec::with_capacity(merged_bytes.len()));
        value.extend_from_slice(merged_bytes);
        Some(value)
    }
}

struct Finisher<F: FnMut(Entry) -> Result<()>> {
    current: HashMap<U32<BE>, u32>,
    value_buf: Vec<u8>,
    file: FileId,
    prev_offset: u32,
    on_entry: F,
}

impl<F: FnMut(Entry) -> Result<()>> Finisher<F> {
    fn new(file: FileId, on_entry: F) -> Self {
        Self {
            file,
            on_entry,
            current: Default::default(),
            value_buf: Default::default(),
            prev_offset: Default::default(),
        }
    }

    fn on_markers(&mut self, offset: u32, markers: &[Marker]) -> Result<()> {
        self.flush(offset)?;

        // get all of the new ends for each entity
        for marker in markers {
            let end = marker.end.get();

            self.current
                .entry(marker.id)
                .and_modify(|prev| {
                    // take the maximum end of the region if it already exists for this
                    // entity
                    *prev = (*prev).max(end);
                })
                .or_insert(end);
        }

        // rather than borrowing all of the other fields just move out `current`
        let mut current = core::mem::take(&mut self.current);

        // delete all of the entities that ended on this marker
        current.retain(|id, end| {
            if *end > offset {
                self.value_buf.extend_from_slice(id.as_bytes());
                true
            } else {
                // the region is complete and no longer needed
                false
            }
        });

        self.current = current;

        Ok(())
    }

    fn flush(&mut self, offset: u32) -> Result<()> {
        debug_assert!(offset >= self.prev_offset);
        let prev_offset = core::mem::replace(&mut self.prev_offset, offset);

        if self.value_buf.is_empty() {
            return Ok(());
        }

        // The ids will be randomized due to hashmap ordering so sort it
        <LayoutVerified<_, [EntityId]>>::new_slice_unaligned(&mut self.value_buf[..])
            .unwrap()
            .into_mut_slice()
            .sort();

        let entry = Entry {
            buf: &self.value_buf,
            start: prev_offset,
            end: offset,
            file: self.file,
        };

        (self.on_entry)(entry)?;

        self.value_buf.clear();

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Entry<'a> {
    pub buf: &'a [u8],
    pub start: u32,
    pub end: u32,
    pub file: FileId,
}

impl<'a> Entry<'a> {
    pub fn ids(&self) -> &[U32<BE>] {
        <LayoutVerified<_, [U32<BE>]>>::new_slice_unaligned(self.buf)
            .unwrap()
            .into_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Sled;

    fn markers() -> Result<Markers> {
        let db = Sled::new()?;
        Ok(Markers::new(db.open_tree([])?))
    }

    #[test]
    fn overlapping_test() -> Result<()> {
        let markers = markers()?;
        let file = FileId::new(1);
        let entity1 = EntityId::new(1);
        let entity2 = EntityId::new(2);

        markers.mark(file, 0..5, entity1)?;
        markers.mark(file, 0..7, entity1)?;
        markers.mark(file, 0..50, entity2)?;
        markers.mark(file, 51..53, entity1)?;

        let mut entries = vec![];
        markers.for_each(file, |entry| {
            let ids: Vec<_> = entry.ids().iter().map(|v| v.get()).collect();
            let range = entry.start..entry.end;
            entries.push((range, ids));
            Ok(())
        })?;

        assert_eq!(
            entries,
            vec![
                (0..5, vec![1, 2]),
                (5..7, vec![1, 2]),
                (7..50, vec![2]),
                (51..53, vec![1]),
            ]
        );

        Ok(())
    }
}
