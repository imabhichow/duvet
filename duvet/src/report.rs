use crate::schema::AnnotationId;
use core::{cmp::Ordering, ops::Range};
use smallvec::SmallVec;

type AnnotationVec = SmallVec<[AnnotationId; 2]>;

#[derive(Default)]
pub struct AnnotationSet {
    annotations: Vec<AnnoRegion>,
}

impl AnnotationSet {
    pub fn insert(&mut self, id: AnnotationId, bytes: Range<u32>) {
        if bytes.start >= bytes.end {
            return;
        }
        self.annotations.push(AnnoRegion {
            id,
            start: bytes.start,
            end: bytes.end,
        });
    }

    pub fn finish(mut self) -> Regions {
        self.annotations.sort();

        dbg!(&self.annotations);

        let mut regions = vec![];

        let mut prev_end = 0;
        for (idx, anno) in self.annotations.iter().copied().enumerate() {
            let AnnoRegion { start, end, id } = anno;

            if prev_end == end {
                continue;
            }

            let start = start.max(prev_end);
            prev_end = end;

            let mut ids = AnnotationVec::new();
            ids.push(id);

            for other in self.annotations[(idx + 1)..].iter() {
                if other.start <= start {
                    ids.push(other.id);
                } else {
                    break;
                }
            }

            regions.push(Region { ids, start, end });
        }

        Regions { regions }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
struct AnnoRegion {
    start: u32,
    end: u32,
    id: AnnotationId,
}

#[derive(Debug)]
pub struct Region {
    ids: AnnotationVec,
    start: u32,
    end: u32,
}

#[derive(Debug)]
pub struct Regions {
    regions: Vec<Region>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked() {
        bolero::check!()
            .with_type::<Vec<(_, Range<u32>)>>()
            .for_each(|regions| {
                let mut set = AnnotationSet::default();

                for (id, bytes) in regions.iter().cloned() {
                    set.insert(AnnotationId(id), bytes);
                }

                let regions = set.finish();

                dbg!(&regions);

                let mut prev_start = None;
                let mut prev_end = 0;
                for region in regions.regions.iter() {
                    assert!(!region.ids.is_empty());
                    assert!(region.start < region.end);
                    assert_ne!(Some(region.start), prev_start);
                    assert!(region.start >= prev_end);
                    prev_start = Some(region.start);
                    prev_end = region.end;
                }
            })
    }
}
