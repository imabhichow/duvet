use crate::{entity::Entities, fs::Fs, region::Regions, reporters::Reporters};
use anyhow::Result;
use sled::{Config, Db as Inner};

macro_rules! ids {
    ($($name:ident),* $(,)?) => {
        ids!([$($name)*], 0u8);
    };
    ([], $value:expr) => {
        // done
    };
    ([$name:ident $($rest:ident)*], $value:expr) => {
        const $name: [u8; 1] = [$value];
        ids!([$($rest)*], $value + 1);
    };
}

ids!(
    FILE_CONTENTS,
    FILE_LINE_TO_OFFSET,
    FILE_OFFSET_TO_LINE,
    FILE_PATH_TO_ID,
    FILE_ID_TO_PATH,
    ATTRIBUTE_ENTITIES,
    ATTRIBUTES,
    ENTITIES,
    ENTITY_REGIONS,
    MARKERS,
    REGIONS,
);

pub struct Db {
    #[allow(dead_code)]
    db: Inner,
    entities: Entities,
    fs: Fs,
    regions: Regions,
    reporters: Reporters,
}

impl Db {
    pub fn new() -> Result<Self> {
        let db = Config::new().temporary(true).open()?;
        let fs = Fs {
            contents: db.open_tree(FILE_CONTENTS)?,
            line2offset: db.open_tree(FILE_LINE_TO_OFFSET)?,
            offset2line: db.open_tree(FILE_OFFSET_TO_LINE)?,
            path2id: db.open_tree(FILE_PATH_TO_ID)?,
            id2path: db.open_tree(FILE_ID_TO_PATH)?,
        };
        let entities = Entities {
            attribute_entities: db.open_tree(ATTRIBUTE_ENTITIES)?,
            attributes: db.open_tree(ATTRIBUTES)?,
            entities: db.open_tree(ENTITIES)?,
        };
        let regions = Regions {
            entity_regions: db.open_tree(ENTITY_REGIONS)?,
            markers: db.open_tree(MARKERS)?,
            regions: db.open_tree(REGIONS)?,
        };
        regions.init();
        let reporters = Reporters::new();
        Ok(Self {
            db,
            entities,
            fs,
            regions,
            reporters,
        })
    }

    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    pub fn fs(&self) -> &Fs {
        &self.fs
    }

    pub fn regions(&self) -> &Regions {
        &self.regions
    }

    pub fn reporters(&self) -> &Reporters {
        &self.reporters
    }
}
