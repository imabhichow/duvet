use crate::{
    db::Db,
    schema::{FileId, InstanceId},
};
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Export {
    pub version: String,

    //#[serde(rename = "type")]
    //pub ty: String,
    pub data: Vec<Data>,
}

impl Export {
    pub fn trim(&mut self) {
        for data in self.data.iter_mut() {
            data.trim();
        }
        self.data.retain(|data: &Data| !data.is_empty())
    }

    pub fn load(&self, db: &Db) -> Result<()> {
        for data in &self.data {
            data.load(db)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub files: Vec<File>,
    pub functions: Vec<Function>,
    pub totals: Summary,
}

impl Data {
    pub fn trim(&mut self) {
        self.files.retain(|f| !f.is_external());
        self.functions.retain(|f| !f.is_empty());
    }

    pub fn is_empty(&self) -> bool {
        let mut is_empty = self.files.is_empty() && self.functions.is_empty();

        is_empty |= self.totals.regions.covered == 0;

        is_empty
    }

    pub fn load(&self, db: &Db) -> Result<()> {
        self.files
            .par_iter()
            .map(|f| f.load(db))
            .chain(self.functions.par_iter().map(|f| f.load(db)))
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct File {
    pub expansions: Vec<Value>, // TODO figure out what goes in here
    pub filename: String,
    pub segments: Vec<Segment>,
    pub summary: Summary,
}

impl File {
    pub fn is_external(&self) -> bool {
        self.filename.starts_with('/')
    }

    pub fn load(&self, db: &Db) -> Result<()> {
        let file = db
            .fs()
            .load_file(Path::new(&self.filename))
            .with_context(|| format!("could not load source file: {:?}", self.filename))?;

        // TODO support instances
        let instance = None;
        for segment in &self.segments {
            segment.load(db, file, instance)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Function {
    pub count: usize,
    pub filenames: Vec<String>,
    #[serde(deserialize_with = "demangle")]
    pub name: String,
    pub regions: Vec<Region>,
}

impl Function {
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.is_external()
    }

    pub fn is_external(&self) -> bool {
        self.filenames
            .iter()
            .all(|filename| filename.starts_with('/'))
    }

    pub fn load(&self, db: &Db) -> Result<()> {
        let files = self
            .filenames
            .iter()
            .map(|file| {
                db.fs()
                    .load_file(Path::new(file))
                    .with_context(|| format!("could not load source file: {:?}", file))
            })
            .collect::<Result<Vec<_>>>()?;
        for file in files {
            let instance = None;
            for region in &self.regions {
                region.load(db, file, instance)?;
            }
        }
        Ok(())
    }
}

fn demangle<'de, D>(de: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <&str>::deserialize(de)?;
    let s = rustc_demangle::demangle(s);
    Ok(format!("{:#}", s))
}

#[derive(Debug, serde_tuple::Deserialize_tuple)]
pub struct Segment {
    pub line: usize,
    pub col: usize,
    pub execution_count: u64,
    pub has_count: bool,
    pub is_region_entry: bool,
    pub is_gap_region: bool,
}

impl Segment {
    pub fn load(&self, db: &Db, file: FileId, instance: Option<InstanceId>) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, serde_tuple::Deserialize_tuple)]
pub struct Region {
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
    pub execution_count: u64,
    pub file_id: usize,
    pub expanded_file_id: usize,
    pub kind: u64,
}

impl Region {
    pub fn load(&self, db: &Db, file: FileId, instance: Option<InstanceId>) -> Result<()> {
        let offsets = db
            .fs()
            .map_line_column(
                file,
                (
                    (self.line_start - 1) as _,
                    (self.col_start.saturating_sub(1)) as _,
                ),
                ((self.line_end - 1) as _, (self.col_end - 1) as _),
            )
            .unwrap();

        let entity = db.entities().create()?;
        db.regions().insert(file, instance, entity, offsets)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Summary {
    pub functions: Total,
    pub instantiations: Total,
    pub lines: Total,
    pub regions: Total,
}

#[derive(Debug, Deserialize)]
pub struct Total {
    pub count: u64,
    pub covered: u64,
    pub percent: f64,
}
