use crate::schema::{EntityId, FileId, IdSet, IdSetExt, InstanceId};
use byteorder::BigEndian as BE;
use core::{fmt, ops::Range};
use sled::{Result, Tree};
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U32};

#[repr(C)]
#[derive(AsBytes, FromBytes, Unaligned, Clone, Copy, Debug)]
struct Marker {
    id: EntityId,
    end: U32<BE>,
}

pub struct Regions {
    /// Stores each entity region consolitated view
    pub(crate) entity_regions: Tree,
    /// Stores each entity report marker (open and close)
    pub(crate) markers: Tree,
    /// Stores the consolitated view of each region that is referenced by 1 or more entity
    pub(crate) regions: Tree,
}

impl Regions {
    pub(crate) fn init(&self) {
        self.markers.set_merge_operator(Self::merge_entry);
    }

    pub fn insert(
        &self,
        file: FileId,
        instance: Option<InstanceId>,
        id: EntityId,
        bytes: Range<u32>,
    ) -> Result<()> {
        let start = bytes.start;
        let end = bytes.end;

        if start >= end {
            return Ok(());
        }

        let entry = Marker {
            id,
            end: U32::new(end),
        };

        let instance = instance.unwrap_or_else(InstanceId::default);

        self.markers
            .merge((file, instance, bytes.start).join(), entry.as_bytes())?;
        self.markers
            .merge((file, instance, bytes.end).join(), entry.as_bytes())?;

        Ok(())
    }

    pub(crate) fn finish_file(&self, file: FileId, instance: Option<InstanceId>) -> Result<()> {
        let mut current: HashMap<EntityId, u32> = HashMap::new();
        let mut value_buf = vec![];
        let mut errors = vec![];
        let instance = instance.unwrap_or_else(InstanceId::default);

        for marker in self
            .markers
            .range((file, instance, 0).join()..=(file, instance, u32::MAX).join())
        {
            let (key, value) = marker?;
            let (_file, _instance, offset): (FileId, InstanceId, u32) = key.keys();

            let entities = <LayoutVerified<_, [Marker]>>::new_slice_unaligned(value.as_ref())
                .unwrap()
                .into_slice();

            for entity in entities {
                let end = entity.end.get();

                current
                    .entry(entity.id)
                    .and_modify(|prev| {
                        // take the maximum end of the region if it already exists for this
                        // entity
                        *prev = (*prev).max(end);
                    })
                    .or_insert(end);
            }

            // delete all of the entities that ended on this marker
            current.retain(|id, end| {
                if *end > offset {
                    value_buf.extend_from_slice(id.as_bytes());
                    true
                } else {
                    // notify the entity that the region is complete
                    errors.push(
                        self.entity_regions
                            .insert((*id, file, instance, offset).join(), vec![])
                            .map(|_| ()),
                    );

                    false
                }
            });

            // TODO convert into error set
            for error in errors.drain(..) {
                error?;
            }

            // The ids will be randomized due to hashmap ordering so sort it
            <LayoutVerified<_, [EntityId]>>::new_slice_unaligned(&mut value_buf[..])
                .unwrap()
                .into_mut_slice()
                .sort();

            let entities = <LayoutVerified<_, [EntityId]>>::new_slice_unaligned(&value_buf[..])
                .unwrap()
                .into_slice();

            // notify all of the annotations of sibling regions
            for entity in entities.iter() {
                self.entity_regions.insert(
                    (*entity, file, instance, offset).join(),
                    value_buf.as_slice(),
                )?;
            }

            // store the consolitated region view at the offset
            self.regions.insert(
                (file, instance, offset).join(),
                core::mem::replace(&mut value_buf, vec![]),
            )?;
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

impl fmt::Debug for Regions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Regions")
            .field("len", &self.regions.len())
            .finish()
    }
}

/*
// TODO add file and instance
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
    pub fn new((key, ids): (IVec, IVec)) -> Self {
        let (_file, _instance, offset): (FileId, InstanceId, _) = key.keys();
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
        let db = crate::db::Db::new().unwrap();
        let set = db.annotations();

        let file = FileId::new(1);
        let instance = None;

        for (type_id, bytes) in regions.iter().cloned() {
            let id = set.insert(&[TypeId::new(type_id)]).unwrap();
            set.insert_region(file, instance, id, bytes).unwrap();
        }

        set.finish_file(file, instance).unwrap();

        Regions::new(set.regions.clone())
    }

    #[test]
    fn checked() {
        bolero::check!()
            .with_type::<Vec<(_, Range<u32>)>>()
            .for_each(|entries| {
                let regions = regions(entries);

                let mut last_count = 0;
                for (idx, entry) in regions.iter().enumerate() {
                    let region = entry.unwrap();
                    if idx == 0 || last_count == 0 {
                        assert!(
                            !region.ids().is_empty(),
                            "empty regions should not start or repeat"
                        );
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
*/
