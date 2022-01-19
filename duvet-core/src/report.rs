use crate::{
    analyze::Output,
    db::Db,
    vfs::{PathId, PathIdMap},
};
use std::{any::Any, ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map(Option<Arc<PathIdMap<List>>>);

impl Default for Map {
    fn default() -> Self {
        Self::empty()
    }
}

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

impl Default for List {
    fn default() -> Self {
        Self::empty()
    }
}

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
pub struct MultiList(Option<Arc<[List]>>);

impl Deref for MultiList {
    type Target = [List];

    fn deref(&self) -> &Self::Target {
        if let Some(list) = self.0.as_deref() {
            list
        } else {
            &[][..]
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiMap(Option<Arc<PathIdMap<MultiList>>>);

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub file: Option<PathId>,
    pub value: Arc<dyn DiagnosticValue>,
}

impl PartialEq for Diagnostic {
    fn eq(&self, other: &Self) -> bool {
        // TODO compare actual values
        self.file.eq(&other.file) && self.value.type_id() == other.value.type_id()
        // && self.value.dyn_eq(&other.value)
    }
}

impl Eq for Diagnostic {}

pub trait DiagnosticValue: Output {
    // TODO
}

pub fn diagnose_path(db: &dyn Db, path_id: PathId) -> MultiList {
    let manifest = db.manifest();
    let path = db.paths().resolve(path_id);

    let mut results = vec![];

    for mappers in manifest.mappers().values() {
        for mapper in mappers {
            if mapper.is_match(&path) {
                let set = db.map_path(path_id, mapper.clone());

                if !set.report.is_empty() {
                    results.push(set.report);
                }

                break;
            }
        }
    }

    for reducer in manifest.reducers().values() {
        let set = db.reduce(reducer.clone());

        if let Some(report) = set.reports.get(path_id) {
            if !report.is_empty() {
                results.push(report.clone());
            }
        }
    }

    if results.is_empty() {
        MultiList(None)
    } else {
        MultiList(Some(Arc::from(results.into_boxed_slice())))
    }
}
