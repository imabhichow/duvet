use crate::{
    linemap::LinesIter,
    schema::{Annotation, AnnotationId, SourceId, SourceLocationId, TypeId},
};
use core::{convert::TryInto, ops::Range};
use rusqlite::{params, Connection, Error, OptionalExtension, Result};
use std::path::Path;

static INIT: &str = include_str!("./schema.sql");

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::new(Connection::open_in_memory()?)
    }

    fn new(conn: Connection) -> Result<Self> {
        conn.execute_batch(INIT)?;

        Ok(Self { conn })
    }

    pub fn add_source(&mut self, source: &str) -> Result<SourceId> {
        let h = hash(source);

        let res = self
            .conn
            .prepare_cached(r#"INSERT OR IGNORE INTO sources (hash) VALUES (?)"#)?
            .insert(&[h.as_ref()]);

        let id = match res {
            Ok(id) => {
                let id = SourceId(id);
                self.add_source_lines(id, source)?;
                id
            }
            Err(Error::StatementChangedRows(_)) => self
                .conn
                .prepare_cached(r#"SELECT id FROM sources WHERE hash = (?)"#)?
                .query_row(&[h.as_ref()], |row| row.get(0))?,
            Err(err) => return Err(err),
        };

        Ok(id)
    }

    fn add_source_lines(&mut self, id: SourceId, source: &str) -> Result<()> {
        let mut s = self.conn.prepare_cached(
            r#"INSERT INTO source_lines (source_id, line, columns, offset) VALUES (?, ?, ?, ?)"#,
        )?;
        for line in LinesIter::new(source) {
            s.execute(params!(
                id,
                line.line() as u32,
                line.len() as u32,
                line.offset() as u32,
            ))?;
        }
        Ok(())
    }

    pub fn add_source_location(
        &mut self,
        id: SourceId,
        location: &str,
        title: Option<&str>,
    ) -> Result<SourceLocationId> {
        let id = self
            .conn
            .prepare_cached(r#"INSERT INTO sources (source_id, location, title) VALUES (?, ?, ?)"#)?
            .insert(params!(id, location, title))?;
        Ok(SourceLocationId(id))
    }

    pub fn new_annotation(&mut self, id: TypeId) -> Result<AnnotationId> {
        let id = self
            .conn
            .prepare_cached(r#"INSERT INTO annotations (type_id, external_id) VALUES (?)"#)?
            .insert(&[id])?;
        Ok(AnnotationId(id))
    }

    pub fn add_relation(
        &mut self,
        source: AnnotationId,
        target: AnnotationId,
        ty: TypeId,
    ) -> Result<()> {
        self.conn
            .prepare_cached(
                r#"
                INSERT OR IGNORE INTO annotation_relations
                (source_id, target_id, type_id)
                VALUES (?, ?, ?)"#,
            )?
            .execute(params!(source, target, ty))?;
        Ok(())
    }

    pub fn add_region(
        &mut self,
        id: AnnotationId,
        range: Range<usize>,
        source_location: SourceLocationId,
    ) -> Result<()> {
        self.conn
            .prepare_cached(
                r#"
                INSERT OR IGNORE INTO annotation_source_regions
                (annotation_id, start_offset, end_offset, source_location_id)
                VALUES (?, ?, ?, ?)"#,
            )?
            .execute(params!(
                id,
                range.start as u32,
                range.end as u32,
                source_location
            ))?;
        Ok(())
    }

    pub fn add_metric(&mut self, id: AnnotationId, name: &str, value: u32) -> Result<()> {
        self.conn
            .prepare_cached(
                r#"
                INSERT OR IGNORE INTO annotation_metrics
                (annotation_id, name, value)
                VALUES (?, ?, ?)"#,
            )?
            .execute(params!(id, name, value))?;
        Ok(())
    }
}

fn hash(source: &str) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let digest = hasher.finalize();
    digest[..16].try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_test() {
        let _ = Db::open_in_memory().unwrap();
    }

    #[test]
    fn load_source() {
        let mut db = Db::open_in_memory().unwrap();

        let foo_id = db.add_source("FOO").unwrap();
        let bar_id = db.add_source("BAR\n").unwrap();

        assert_ne!(foo_id, bar_id);
        let baz_id = db.add_source("BAZ\n\r").unwrap();

        let foo2_id = db.add_source("FOO").unwrap();
        assert_eq!(foo_id, foo2_id);
    }
}
