use crate::{attribute::Dependency, db::Db, schema::ReporterId};
use anyhow::Result;
use core::{any, fmt};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct Reporters(Arc<Mutex<Inner>>);

impl Reporters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, t: Box<dyn Reporter>) -> ReporterId {
        self.0.lock().unwrap().insert(t)
    }

    pub fn insert_once<F: FnOnce() -> T, T: Reporter>(&self, f: F) -> ReporterId {
        self.0.lock().unwrap().insert_once(f)
    }

    pub fn take(&self) -> HashMap<ReporterId, Box<dyn Reporter>> {
        let mut inner = self.0.lock().unwrap();
        inner.ids.clear();
        core::mem::take(&mut inner.reporters)
    }
}

#[derive(Default)]
struct Inner {
    ids: HashMap<any::TypeId, ReporterId>,
    reporters: HashMap<ReporterId, Box<dyn Reporter>>,
}

impl Inner {
    fn insert(&mut self, t: Box<dyn Reporter>) -> ReporterId {
        let id = ReporterId::new(self.reporters.len() as _);
        self.reporters.insert(id, t);
        id
    }

    fn insert_once<F: FnOnce() -> T, T: Reporter>(&mut self, f: F) -> ReporterId {
        let tid = any::TypeId::of::<T>();

        let entry = self.ids.entry(tid);
        if let Entry::Occupied(v) = &entry {
            return *v.get();
        }
        let id = ReporterId::new(self.reporters.len() as _);
        entry.or_insert(id);
        self.reporters.insert(id, Box::new(f()));
        id
    }
}

pub trait Reporter: 'static + Send {
    fn updates(&self) -> &[Dependency] {
        &[]
    }

    fn dependencies(&self) -> &[Dependency] {
        &[]
    }

    fn report(&mut self, db: &Db) -> Result<()>;
}

impl fmt::Debug for Reporters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Reporters")
            .field("len", &self.0.lock().unwrap().reporters.len())
            .finish()
    }
}
