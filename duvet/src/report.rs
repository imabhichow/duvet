use crate::schema::AnnotationId;
use byteorder::BigEndian as BE;
use core::{convert::TryInto, fmt, ops::Range};
use sled::{IVec, Result, Tree};
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U32};

#[repr(C)]
#[derive(AsBytes, FromBytes, Unaligned, Clone, Copy, Debug)]
struct AnnotationEntry {
    id: AnnotationId,
    end: U32<BE>,
}

pub struct AnnotationSet {
    annotations: Tree,
}

impl AnnotationSet {
    pub fn new(annotations: Tree) -> Self {
        annotations.set_merge_operator(Self::merge_entry);

        Self { annotations }
    }

    pub fn insert(&mut self, id: AnnotationId, bytes: Range<u32>) -> Result<()> {
        let start = bytes.start;
        let end = bytes.end;

        if start >= end {
            return Ok(());
        }

        let entry = AnnotationEntry {
            id,
            end: U32::new(end),
        };

        self.annotations
            .merge(bytes.start.to_be_bytes(), entry.as_bytes())?;
        self.annotations
            .merge(bytes.end.to_be_bytes(), entry.as_bytes())?;

        Ok(())
    }

    pub fn finish(self, out: &Tree) -> Result<()> {
        let mut current: HashMap<AnnotationId, u32> = HashMap::new();
        let mut value_buf = vec![];

        for entry in self.annotations.iter() {
            let (key, value) = entry?;
            let offset = get_offset(&key);

            let entries =
                <LayoutVerified<_, [AnnotationEntry]>>::new_slice_unaligned(value.as_ref())
                    .unwrap()
                    .into_slice();

            for entry in entries {
                let end = entry.end.get();

                current
                    .entry(entry.id)
                    .and_modify(|prev| *prev = (*prev).max(end))
                    .or_insert(end);
            }

            // delete all of the annotations that ended on this entry
            current.retain(|anno_id, end| {
                if *end > offset {
                    value_buf.extend_from_slice(anno_id.as_bytes());
                    true
                } else {
                    false
                }
            });

            out.insert(key, value_buf.as_slice())?;

            value_buf.clear();
        }

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

fn get_offset<V: AsRef<[u8]>>(key: V) -> u32 {
    u32::from_be_bytes(key.as_ref().try_into().unwrap())
}

struct Regions(Tree);

impl Regions {
    pub fn new(regions: Tree) -> Self {
        Self(regions)
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<Region>> + '_ {
        self.0.iter().map(|v| v.map(Region::new))
    }
}

impl fmt::Debug for Regions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.iter().filter_map(|r| r.ok()))
            .finish()
    }
}

struct Region {
    pub offset: u32,
    ids: IVec,
}

impl fmt::Debug for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Region")
            .field("offset", &self.offset)
            .field("ids", &self.ids())
            .finish()
    }
}

impl Region {
    pub fn new((offset, ids): (IVec, IVec)) -> Self {
        let offset = get_offset(offset);
        Self { offset, ids }
    }

    pub fn ids(&self) -> &[AnnotationId] {
        <LayoutVerified<_, [AnnotationId]>>::new_slice_unaligned(self.ids.as_ref())
            .unwrap()
            .into_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(regions: &[(u32, Range<u32>)]) -> Regions {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let annos = db.open_tree("annos").unwrap();

        let mut set = AnnotationSet::new(annos);

        for (id, bytes) in regions.iter().cloned() {
            set.insert(AnnotationId::new(id), bytes).unwrap();
        }

        let regions = db.open_tree("regions").unwrap();

        set.finish(&regions).unwrap();

        Regions::new(regions)
    }

    #[test]
    fn checked() {
        bolero::check!()
            .with_type::<Vec<(_, Range<u32>)>>()
            .for_each(|entries| {
                let regions = regions(entries);

                let mut last_count = 0;
                for entry in regions.iter() {
                    let region = entry.unwrap();
                    if last_count == 0 {
                        assert!(!region.ids().is_empty(), "empty regions should not repeat");
                    }
                    last_count = region.ids().len();
                }
            });
    }

    #[test]
    fn overlap() {
        insta::assert_debug_snapshot!(regions(&[(1, 0..2), (2, 2..4), (3, 0..5)]));
    }
}
