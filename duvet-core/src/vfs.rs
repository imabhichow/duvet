use crate::{
    db::Db,
    error::{Error, Result},
    intern::{self, Intern},
};
use arcstr::{ArcStr, Substr};
use bytes::Bytes;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Default)]
pub struct Paths(Arc<Intern<PathBuf>>);

impl Paths {
    pub fn intern(&self, path: &Path) -> PathId {
        PathId(self.0.intern(path))
    }

    pub fn resolve(&self, path_id: PathId) -> intern::Ref<PathBuf> {
        self.0.resolve(path_id.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PathId(intern::Id);

// TODO change the default hasher
pub type PathIdMap<V> = std::collections::HashMap<PathId, V>;
pub type PathIdIter<'a, V> = std::collections::hash_map::Iter<'a, PathId, V>;

pub trait Filesystem {
    fn paths(&self) -> &Paths;

    fn fs_read(&self, path: &Path) -> Node {
        fs_read(self.paths(), path)
    }

    fn fs_watch(&self, path: &Path) {
        // noop
        let _ = path;
    }
}

pub fn fs_read(paths: &Paths, path: &Path) -> Node {
    if path.is_file() {
        fs_read_file(paths, path)
    } else {
        fs_read_dir(paths, path)
    }
}

pub fn fs_read_file(paths: &Paths, path: &Path) -> Node {
    let id = paths.intern(path);

    let result = (|| {
        let v = std::fs::read_to_string(path)?;
        let v = ArcStr::from(v);
        Ok(Node::String(id, v))
    })();

    match result {
        Ok(v) => v,
        Err(err) => Node::Error(id, err),
    }
}

pub fn fs_read_dir(paths: &Paths, path: &Path) -> Node {
    let id = paths.intern(path);

    let result = (|| {
        let dir = fs::read_dir(path)?;
        let children = dir
            .map(|res| match res {
                Ok(entry) => {
                    let path = entry.path();
                    let path: &Path = &path;
                    let id = paths.intern(path);
                    Ok(id)
                }
                Err(err) => Err(err.into()),
            })
            .collect();

        Ok(children)
    })();

    match result {
        Ok(v) => Node::Directory(id, v),
        Err(err) => Node::Error(id, err),
    }
}

impl Node {
    pub fn as_str(&self) -> Result<&ArcStr> {
        match &*self {
            Node::String(_, v) => Ok(v),
            Node::Binary(_, _) => Err("file is not valid utf8".into()),
            Node::Directory(_, _) => Err("trying to read a directory as a file".into()),
            Node::Error(_, e) => Err(e.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    String(PathId, ArcStr),
    Binary(PathId, Bytes),
    Directory(PathId, Arc<[Result<PathId, Error>]>),
    Error(PathId, Error),
}

pub fn vfs_read(db: &dyn Db, path_id: PathId) -> Node {
    db.salsa_runtime()
        .report_synthetic_read(salsa::Durability::LOW);

    let paths = db.paths();
    let path = paths.resolve(path_id);

    db.fs_read(&path)
}
