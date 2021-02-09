use crate::{
    schema::{FileId, IdSet, IdSetExt},
    source::Loader,
};
use anyhow::Result;
use sled::{
    transaction::{
        ConflictableTransactionError, ConflictableTransactionResult, TransactionError,
        TransactionalTree,
    },
    Transactional, Tree,
};
use std::{io::BufRead, path::Path};
use zerocopy::AsBytes;

pub struct Fs {
    pub(crate) contents: Tree,
    pub(crate) line2offset: Tree,
    pub(crate) offset2line: Tree,
    pub(crate) path2id: Tree,
    pub(crate) id2path: Tree,
}

pub struct Transaction<'a> {
    contents: &'a TransactionalTree,
    line2offset: &'a TransactionalTree,
    offset2line: &'a TransactionalTree,
    path2id: &'a TransactionalTree,
    id2path: &'a TransactionalTree,
}

impl Fs {
    pub fn load_file(&self, path: &Path) -> Result<FileId> {
        self.load(path.to_string_lossy(), |_| {
            let file = std::fs::File::open(path)?;
            let file = std::io::BufReader::new(file);
            Ok(file)
        })
    }

    #[cfg(feature = "fetch")]
    pub fn load_url(&self, url: &str) -> Result<FileId> {
        self.load(url, |_| {
            use reqwest::blocking::get;
            let res = get(url)?;
            let res = std::io::BufReader::new(res);
            Ok(res)
        })
    }

    pub fn load<P: AsRef<str>, F: Fn(&P) -> Result<R>, R>(&self, path: P, load: F) -> Result<FileId>
    where
        R: BufRead,
    {
        self.transaction(|t| {
            let path_str = path.as_ref();

            // short-cut loading
            if let Some(id) = t.path2id.get(path_str)? {
                let (id,) = id.keys();
                return Ok(id);
            }

            let mut reader = load(&path).map_err(ConflictableTransactionError::Abort)?;

            // 32 bits should be plenty
            let id = t.path2id.generate_id()? as u32;
            let id: FileId = id.into();

            t.path2id.insert(path_str, id)?;
            t.id2path.insert(id, path_str)?;

            let mut loader = Loader::new(&mut reader);

            let mut linenum = 0u32;
            while let Some(res) = loader.next() {
                let line = res.map_err(|e| ConflictableTransactionError::Abort(e.into()))?;
                t.line2offset
                    .insert(&(id, linenum).join(), line.as_bytes())?;
                t.offset2line
                    .insert(&(id, line.offset).join(), &linenum.to_be_bytes())?;
                linenum += 1;
            }

            t.contents.insert(id, loader.contents.into_bytes())?;

            Ok(id)
        })
    }

    fn transaction<F: Fn(Transaction) -> ConflictableTransactionResult<T, anyhow::Error>, T>(
        &self,
        f: F,
    ) -> Result<T> {
        let v = (
            &self.contents,
            &self.line2offset,
            &self.offset2line,
            &self.path2id,
            &self.id2path,
        )
            .transaction(
                move |(contents, line2offset, offset2line, path2id, id2path)| {
                    f(Transaction {
                        contents,
                        line2offset,
                        offset2line,
                        path2id,
                        id2path,
                    })
                },
            );

        match v {
            Ok(v) => Ok(v),
            Err(TransactionError::Abort(e)) => Err(e),
            Err(TransactionError::Storage(e)) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const SELF: &str = include_str!("./fs.rs");

    #[test]
    fn vfs() {
        let db = crate::db::Db::new().unwrap();

        let first = db.fs().load(file!(), |_| Ok(Cursor::new(SELF))).unwrap();
        let second = db.fs().load(file!(), |_| Ok(Cursor::new(SELF))).unwrap();

        assert_eq!(first, second);
    }
}
