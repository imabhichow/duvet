use crate::{
    db::Db,
    vfs::{PathId, PathIdMap},
};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map(Option<Arc<PathIdMap<List>>>);

impl Map {
    pub const fn empty() -> Self {
        Self(None)
    }

    pub fn is_empty(&self) -> bool {
        if let Some(v) = self.0.as_ref() {
            v.is_empty()
        } else {
            true
        }
    }

    pub fn get(&self, path_id: PathId) -> Option<&List> {
        let map = self.0.as_ref()?;
        map.get(&path_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List(Option<Arc<[Diagnostic]>>);

impl List {
    pub const fn empty() -> Self {
        Self(None)
    }

    pub fn is_empty(&self) -> bool {
        if let Some(v) = self.0.as_ref() {
            v.is_empty()
        } else {
            true
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Set(Arc<[List]>);

impl Deref for Set {
    type Target = [List];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub file: PathId,
}

pub fn diagnose_path(db: &dyn Db, path_id: PathId) -> Set {
    let manifest = db.manifest();
    let path = db.paths().resolve(path_id);

    let mut results = vec![];

    for mapper in manifest.mappers().values() {
        if mapper.is_match(&path) {
            let set = db.map_path(path_id, mapper.clone());

            if !set.report.is_empty() {
                results.push(set.report);
            }
        }
    }

    for reducer in manifest.reducers().values() {
        let set = db.reduce(reducer.clone());

        if let Some(report) = set.reports.get(path_id) {
            results.push(report.clone());
        }
    }

    Set(Arc::from(results.into_boxed_slice()))
}
