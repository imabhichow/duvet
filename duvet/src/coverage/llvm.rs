use crate::{
    db::Db,
    schema::{EntityId, FileId},
    types::EXECUTIONS,
};
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

pub trait EntityVisitor: Sync {
    fn on_entity(&self, file: FileId, entity: EntityId) -> Result<()>;
}

pub struct FnVisitor<F: Sync + Fn(FileId, EntityId) -> Result<()>>(pub F);

impl<F: Sync + Fn(FileId, EntityId) -> Result<()>> EntityVisitor for FnVisitor<F> {
    fn on_entity(&self, file: FileId, entity: EntityId) -> Result<()> {
        (self.0)(file, entity)
    }
}

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

    pub fn load<V: EntityVisitor>(&self, db: &Db, visitor: &V) -> Result<()> {
        for data in &self.data {
            data.load(db, visitor)?;
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

    pub fn load<V: EntityVisitor>(&self, db: &Db, visitor: &V) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        self.files
            .par_iter()
            .map(|f| f.load(db))
            .chain(self.functions.par_iter().map(|f| f.load(db, visitor)))
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
        if self.is_external() {
            return Ok(());
        }

        let file = db
            .fs()
            .load_file(Path::new(&self.filename))
            .with_context(|| format!("could not load source file: {:?}", self.filename))?;

        for segment in &self.segments {
            segment.load(db, file)?;
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

    pub fn load<V: EntityVisitor>(&self, db: &Db, visitor: &V) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

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
            for region in &self.regions {
                region.load(db, file, visitor)?;
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
    pub fn load(&self, db: &Db, file: FileId) -> Result<()> {
        // TODO
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
    pub fn load<V: EntityVisitor>(&self, db: &Db, file: FileId, visitor: &V) -> Result<()> {
        if self.execution_count == 0 {
            return Ok(());
        }

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
        db.entities()
            .set_attribute(entity, EXECUTIONS, self.execution_count)?;
        visitor.on_entity(file, entity)?;
        db.regions().insert(file, offsets, entity)?;

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
