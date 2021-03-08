use crate::{db::Db, schema::FileId, source::LinesIter};
use anyhow::Result;

pub struct Config {
    outdir: String,
}

impl Config {
    pub fn file(&self, db: &Db, file: FileId) -> Result<()> {
        let contents = db.fs().open(file)?;

        for line in LinesIter::new(&contents) {
            //
        }

        Ok(())
    }
}
