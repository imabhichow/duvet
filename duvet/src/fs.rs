use crate::{
    schema::{FileId, IdSet, IdSetExt},
    source::Loader,
};
use anyhow::{anyhow, Result};
use byteorder::BigEndian as BE;
use core::fmt;
use sled::{
    transaction::{
        ConflictableTransactionError, ConflictableTransactionResult, TransactionError,
        TransactionalTree,
    },
    Transactional, Tree,
};
use std::{io::BufRead, path::Path};
use zerocopy::{AsBytes, LayoutVerified, U32};

pub type Id = FileId;

pub struct Fs {
    pub(crate) contents: Tree,
    pub(crate) line_to_offset: Tree,
    pub(crate) offset_to_line: Tree,
    pub(crate) path_to_id: Tree,
    pub(crate) id_to_path: Tree,
}

pub struct Transaction<'a> {
    contents: &'a TransactionalTree,
    line_to_offset: &'a TransactionalTree,
    offset_to_line: &'a TransactionalTree,
    path_to_id: &'a TransactionalTree,
    id_to_path: &'a TransactionalTree,
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
            if let Some(id) = t.path_to_id.get(path_str)? {
                let (id,) = id.keys();
                return Ok(id);
            }

            let mut reader = load(&path).map_err(ConflictableTransactionError::Abort)?;

            // 32 bits should be plenty
            let id = t.path_to_id.generate_id()? as u32;
            let id: FileId = id.into();

            t.path_to_id.insert(path_str, id)?;
            t.id_to_path.insert(id, path_str)?;

            let mut loader = Loader::new(&mut reader);

            let mut linenum = 0u32;
            while let Some(res) = loader.next() {
                let line = res.map_err(|e| ConflictableTransactionError::Abort(e.into()))?;

                let mut v = Vec::with_capacity(line.len() as usize);

                let base = line.offset();
                for (col, _) in loader.contents[line.range_usize()].char_indices() {
                    v.extend_from_slice(&(base + col as u32).to_be_bytes());
                }
                v.extend_from_slice(&(base + line.len()).to_be_bytes());

                t.line_to_offset.insert(&(id, linenum).join(), v)?;
                linenum += 1;
            }

            t.contents.insert(id, loader.contents.into_bytes())?;

            Ok(id)
        })
    }

    pub fn open(&self, file: FileId) -> Result<IStr> {
        let contents = self.contents.get(file)?;
        if let Some(contents) = contents {
            Ok(IStr(contents))
        } else {
            Err(anyhow!("could not find file {:?}", file))
        }
    }

    pub fn line_offsets(&self, file: FileId, line: u32) -> Result<LineOffsets> {
        let offset = self.line_to_offset.get(&(file, line).join())?;
        if let Some(offset) = offset {
            Ok(LineOffsets(offset))
        } else {
            Err(anyhow!("could not file line {} in file {:?}", line, file))
        }
    }

    pub fn map_line_column(
        &self,
        file: FileId,
        start: (u32, u32),
        end: (u32, u32),
    ) -> Result<core::ops::Range<u32>> {
        let start_offsets = self.line_offsets(file, start.0)?;

        let start_offset = start_offsets
            .get(start.1 as usize)
            .ok_or_else(|| {
                anyhow!(
                    "invalid start column. max: {}, got: {}",
                    start_offsets.len(),
                    start.1
                )
            })?
            .get();

        let end_offset = if start.0 == end.0 {
            start_offsets
                .get(end.1 as usize)
                .ok_or_else(|| {
                    anyhow!(
                        "invalid end column. max: {}, got: {}",
                        start_offsets.len(),
                        end.1
                    )
                })?
                .get()
        } else {
            let end_offsets = self.line_offsets(file, end.0)?;
            end_offsets
                .get(end.1 as usize)
                .ok_or_else(|| {
                    anyhow!(
                        "invalid end column. max: {}, got: {}",
                        end_offsets.len(),
                        end.1
                    )
                })?
                .get()
        };

        Ok(start_offset..end_offset)
    }

    pub fn iter(&self) -> Iter {
        Iter(self.id_to_path.iter())
    }

    fn transaction<F: Fn(Transaction) -> ConflictableTransactionResult<T, anyhow::Error>, T>(
        &self,
        f: F,
    ) -> Result<T> {
        let v = (
            &self.contents,
            &self.line_to_offset,
            &self.offset_to_line,
            &self.path_to_id,
            &self.id_to_path,
        )
            .transaction(
                move |(contents, line_to_offset, offset_to_line, path_to_id, id_to_path)| {
                    f(Transaction {
                        contents,
                        line_to_offset,
                        offset_to_line,
                        path_to_id,
                        id_to_path,
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

impl fmt::Debug for Fs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = f.debug_struct("Fs");

        s.field("len", &self.id_to_path.len());

        // TODO list files in alternate

        s.finish()
    }
}

pub struct LineOffsets(sled::IVec);

impl core::ops::Deref for LineOffsets {
    type Target = [U32<BE>];

    fn deref(&self) -> &Self::Target {
        LayoutVerified::new_slice_unaligned(self.0.as_ref())
            .unwrap()
            .into_slice()
    }
}

pub struct Iter(sled::Iter);

impl Iterator for Iter {
    type Item = Result<(FileId, IStr)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next()? {
            Ok((k, v)) => {
                let (k,) = k.keys();
                let v = IStr(v);

                Some(Ok((k, v)))
            }
            Err(err) => Some(Err(err.into())),
        }
    }
}

pub struct IStr(sled::IVec);

impl core::ops::Deref for IStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { core::str::from_utf8_unchecked(self.0.as_ref()) }
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
