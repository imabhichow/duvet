use crate::{
    entity::Entities, fs::Fs, marker::Markers, notification::Notifications, region::Regions,
    schema::IdSetExt,
};
use anyhow::Result;
use core::fmt;
use rayon::prelude::*;
use tempdir::TempDir;

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
    REGION_MARKERS,
    NOTIFICATION_MARKERS,
    NOTIFICATION_REGIONS,
);

pub struct Db {
    #[allow(dead_code)]
    db: Sled,
    entities: Entities,
    fs: Fs,
    regions: Regions,
    notifications: Notifications,
}

impl Db {
    pub fn new() -> Result<Self> {
        let db = Sled::new()?;

        let fs = Fs {
            contents: db.open_tree(FILE_CONTENTS)?,
            line_to_offset: db.open_tree(FILE_LINE_TO_OFFSET)?,
            offset_to_line: db.open_tree(FILE_OFFSET_TO_LINE)?,
            path_to_id: db.open_tree(FILE_PATH_TO_ID)?,
            id_to_path: db.open_tree(FILE_ID_TO_PATH)?,
        };
        let entities = Entities {
            attribute_entities: db.open_tree(ATTRIBUTE_ENTITIES)?,
            attributes: db.open_tree(ATTRIBUTES)?,
            entities: db.open_tree(ENTITIES)?,
        };
        let regions = Regions {
            entity_regions: db.open_tree(ENTITY_REGIONS)?,
            markers: Markers::new(db.open_tree(REGION_MARKERS)?),
        };
        let notifications = Notifications::new(
            Markers::new(db.open_tree(NOTIFICATION_MARKERS)?),
            db.open_tree(NOTIFICATION_REGIONS)?,
        );
        entities.init();

        Ok(Self {
            db,
            entities,
            fs,
            regions,
            notifications,
        })
    }

    pub fn finish_regions(&self) -> Result<()> {
        let files = self
            .fs()
            .id_to_path
            .iter()
            .keys()
            .map(|f| {
                let (f,) = f?.keys();
                Ok(f)
            })
            .collect::<Result<Vec<_>>>()?;

        let regions = self.regions();

        files
            .par_iter()
            .map(|file| {
                regions.finish_file(*file)?;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    pub fn finish_notifications(&self) -> Result<()> {
        let files = self
            .fs()
            .id_to_path
            .iter()
            .keys()
            .map(|f| {
                let (f,) = f?.keys();
                Ok(f)
            })
            .collect::<Result<Vec<_>>>()?;

        let notifications = self.notifications();

        files
            .par_iter()
            .map(|file| {
                notifications.finish_file(*file)?;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
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

    pub fn notifications(&self) -> &Notifications {
        &self.notifications
    }
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Db")
            .field("entities", &self.entities())
            .field("fs", &self.fs())
            .field("regions", &self.regions())
            .finish()
    }
}

pub(crate) struct Sled {
    #[allow(dead_code)]
    dir: TempDir,
    db: sled::Db,
}

impl Sled {
    pub fn new() -> Result<Self> {
        let dir = TempDir::new("duvet")?;

        let db = sled::Config::new()
            .path(dir.path())
            .mode(sled::Mode::HighThroughput)
            .temporary(true)
            .open()?;

        Ok(Self { dir, db })
    }
}

impl core::ops::Deref for Sled {
    type Target = sled::Db;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}
