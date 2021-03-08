use crate::{
    marker::Markers,
    schema::{EntityId, FileId, IdSet, IdSetExt},
};
use anyhow::Result;
use core::{fmt, ops::Range};
use sled::{IVec, Tree};
use zerocopy::LayoutVerified;

pub struct Regions {
    /// Stores each entity report marker (open and close)
    pub(crate) markers: Markers,
    /// Stores each entity region consolitated view
    pub(crate) entity_regions: Tree,
}

impl Regions {
    pub fn insert(&self, file: FileId, bytes: Range<u32>, id: EntityId) -> Result<()> {
        self.markers.mark(file, bytes, id)
    }

    pub(crate) fn finish_file(&self, file: FileId) -> Result<()> {
        let entity_regions = &self.entity_regions;

        self.markers.for_each(file, |entry| {
            // notify all of the entities of overlapping regions
            for entity in entry.ids() {
                entity_regions.insert(
                    (*entity, entry.file, entry.start, entry.end).join(),
                    entry.buf,
                )?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn references(&self, entity: EntityId) -> References {
        References(self.entity_regions.scan_prefix(entity))
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
            .field("len", &self.entity_regions.len())
            .finish()
    }
}

pub struct References(sled::Iter);

impl Iterator for References {
    type Item = Result<Reference>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.0.next()? {
            Ok((k, entities)) => {
                let (_, file, start, end): (EntityId, _, _, _) = k.keys();
                Ok(Reference {
                    file,
                    start,
                    end,
                    entities,
                })
            }
            Err(err) => Err(err.into()),
        })
    }
}

pub struct Reference {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
    entities: IVec,
}

impl Reference {
    pub fn entities(&self) -> &[EntityId] {
        <LayoutVerified<_, [EntityId]>>::new_slice_unaligned(&self.entities[..])
            .unwrap()
            .into_slice()
    }

    pub fn range(&self) -> Range<u32> {
        self.start..self.end
    }
}
