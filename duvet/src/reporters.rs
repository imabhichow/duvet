use crate::{db::Db, schema::ReporterId};
use anyhow::Result;
use core::any;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct Reporters(Arc<RwLock<Inner>>);

impl Reporters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, t: Arc<dyn Reporter>) -> ReporterId {
        self.0.write().unwrap().insert(t)
    }

    pub fn insert_once<F: FnOnce() -> T, T: Reporter>(&self, f: F) -> ReporterId {
        self.0.write().unwrap().insert_once(f)
    }
}

#[derive(Default)]
struct Inner {
    ids: HashMap<any::TypeId, ReporterId>,
    types: HashMap<ReporterId, Arc<dyn Reporter>>,
}

impl Inner {
    fn insert(&mut self, t: Arc<dyn Reporter>) -> ReporterId {
        let id = ReporterId::new(self.types.len() as _);
        self.types.insert(id, t);
        id
    }

    fn insert_once<F: FnOnce() -> T, T: Reporter>(&mut self, f: F) -> ReporterId {
        let tid = any::TypeId::of::<T>();

        let entry = self.ids.entry(tid);
        if let Entry::Occupied(v) = &entry {
            return *v.get();
        }
        let id = ReporterId::new(self.types.len() as _);
        entry.or_insert(id);
        self.types.insert(id, Arc::new(f()));
        id
    }
}

pub trait Reporter: 'static + Send {
    fn dependencies(&self) -> &[ReporterId] {
        &[]
    }

    fn report(&self, db: &Db) -> Result<()>;
}
